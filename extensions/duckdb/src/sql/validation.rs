//! SQL security validation functions for path and identifier safety.
//!
//! # Helper composition
//!
//! Each helper guards a different threat surface; many sites need more than one.
//!
//! - [`validate_identifier`] / [`quoted_ident`] — for any identifier interpolated
//!   into SQL (table, column, view names). `quoted_ident` is preferred at call
//!   sites because it cannot be invoked without validation.
//! - [`validate_path_chars`] — for path-like strings used either as bound
//!   parameters or interpolated. Catches dangerous shell/SQL metacharacters
//!   and control codes.
//! - [`validate_no_traversal`] — for path-like strings whose semantics depend
//!   on staying inside a specific root. Reject `..` segments before relying
//!   on `starts_with` joins or filesystem reads.
//! - [`escape_sql_string`] / [`sanitize_path_for_sql`] — low-level escaping
//!   used inside [`prepare_path_for_sql`]; not safe to call alone.
//! - [`prepare_path_for_sql`] — the only path helper safe to call standalone
//!   for a value that will be string-interpolated into SQL. Combines the
//!   three checks plus escaping.
//! - [`validate_extra_opts`] — for the `read_json_auto(...)` extra options
//!   fragment, which is interpolated rather than parameterized.
//!
//! Bound-parameter values still benefit from `validate_path_chars` and
//! `validate_no_traversal` for **semantic** correctness (e.g., preventing
//! traversal-based mismatches), even though they are not at risk of
//! injection.

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SqlError {
    #[error("invalid character in path: {0:?}")]
    InvalidPathChar(char),
    #[error("path traversal not allowed: {}", .0.display())]
    PathTraversalNotAllowed(std::path::PathBuf),
    #[error("invalid SQL identifier: {0:?}")]
    InvalidIdentifier(String),
    #[error("invalid extra_opts fragment: {0:?}")]
    InvalidExtraOpts(String),
    #[error("path is not valid UTF-8: {0:?}")]
    InvalidUtf8Path(std::ffi::OsString),
    #[error("path is empty")]
    EmptyPath,
}

/// Validate that a string is a safe SQL identifier (`[a-zA-Z_][a-zA-Z0-9_]*`).
///
/// Used for table names and other identifiers that must be interpolated into SQL.
/// All current call sites pass `&'static str` literals, but this provides
/// defense-in-depth against future misuse.
pub fn validate_identifier(name: &str) -> Result<(), SqlError> {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return Err(SqlError::InvalidIdentifier(name.to_string()));
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return Err(SqlError::InvalidIdentifier(name.to_string()));
    }
    for ch in chars {
        if !ch.is_ascii_alphanumeric() && ch != '_' {
            return Err(SqlError::InvalidIdentifier(name.to_string()));
        }
    }
    Ok(())
}

/// SEC-12 / TASK-0856: const-validated wrapper for SQL identifiers.
///
/// Construct with [`TableName::from_static`] (compile-time validation via
/// `const fn` + `assert!`) so an invalid literal is a build error, not a
/// runtime `quoted_ident` failure. Carries the validated `&'static str`
/// for diagnostics; the quoted form is built on demand and is safe to
/// interpolate into SQL without re-validation.
#[derive(Debug, Clone, Copy)]
pub struct TableName(&'static str);

impl TableName {
    /// Const-validating constructor: panics at compile time if `s` is not
    /// a valid SQL identifier (`[A-Za-z_][A-Za-z0-9_]*`). Designed to be
    /// called from `const fn` constructors so the static-table-name
    /// invariant is enforced at build time.
    #[must_use]
    pub const fn from_static(s: &'static str) -> Self {
        assert!(
            is_valid_identifier_const(s),
            "TableName::from_static requires a valid SQL identifier ([A-Za-z_][A-Za-z0-9_]*)"
        );
        Self(s)
    }

    /// Recover the validated identifier text (e.g. for diagnostics).
    #[must_use]
    pub const fn as_str(&self) -> &'static str {
        self.0
    }

    /// Render the double-quoted SQL form. Safe to interpolate directly
    /// because the identifier was validated at construction.
    #[must_use]
    pub fn quoted(&self) -> String {
        format!("\"{}\"", self.0)
    }
}

const fn is_valid_identifier_const(s: &str) -> bool {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let first = bytes[0];
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    let mut i = 1;
    while i < bytes.len() {
        let b = bytes[i];
        if !(b.is_ascii_alphanumeric() || b == b'_') {
            return false;
        }
        i += 1;
    }
    true
}

/// Validate `name` and return a double-quoted SQL identifier in one step.
///
/// Use this helper at every site that interpolates a table or column name into
/// a SQL string — it guarantees the identifier is validated before quoting,
/// closing off forgotten-validation regressions.
pub fn quoted_ident(name: &str) -> Result<String, SqlError> {
    validate_identifier(name)?;
    Ok(format!("\"{name}\""))
}

/// Validate `extra_opts` fragment for `read_json_auto(...)`.
///
/// Allows only `key=value` pairs (and comma-separated lists of them) where the
/// key is `[a-zA-Z_][a-zA-Z0-9_]*` and the value is a non-negative decimal
/// integer or a bare alphanumeric token. Quotes, parentheses, semicolons, and
/// whitespace are rejected to prevent SQL fragment injection.
///
/// SEC-33 / TASK-1241: hard upper bounds protect the only fragment that
/// gets *interpolated* (rather than parameterized) into
/// `read_json_auto(..., {opts})`. The whole-string cap at
/// [`EXTRA_OPTS_MAX_BYTES`] (4 KiB) and the pair-count cap at
/// [`EXTRA_OPTS_MAX_PAIRS`] (32) bound resource exposure on the
/// interpolated surface. Today's call sites are all static literals well
/// under these caps; the limits document the safety contract for any
/// future dynamic caller — without them an allowlist-conformant
/// multi-megabyte input would pass char-by-char validation and reach the
/// SQL builder unbounded.
pub fn validate_extra_opts(opts: &str) -> Result<(), SqlError> {
    if opts.is_empty() {
        return Err(SqlError::InvalidExtraOpts(opts.to_string()));
    }
    if opts.len() > EXTRA_OPTS_MAX_BYTES {
        return Err(SqlError::InvalidExtraOpts(opts.to_string()));
    }
    let mut pair_count = 0usize;
    for pair in opts.split(',') {
        pair_count += 1;
        if pair_count > EXTRA_OPTS_MAX_PAIRS {
            return Err(SqlError::InvalidExtraOpts(opts.to_string()));
        }
        let mut parts = pair.splitn(2, '=');
        let key = parts
            .next()
            .ok_or_else(|| SqlError::InvalidExtraOpts(opts.to_string()))?;
        let value = parts
            .next()
            .ok_or_else(|| SqlError::InvalidExtraOpts(opts.to_string()))?;
        if parts.next().is_some() {
            return Err(SqlError::InvalidExtraOpts(opts.to_string()));
        }
        validate_identifier(key).map_err(|_| SqlError::InvalidExtraOpts(opts.to_string()))?;
        if value.is_empty() || !value.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(SqlError::InvalidExtraOpts(opts.to_string()));
        }
    }
    Ok(())
}

/// SEC-33 / TASK-1241: hard upper bound on the byte length of an
/// `extra_opts` fragment. Sized well above realistic static call-site
/// values (today's longest is on the order of 100 bytes) so the cap
/// never fires for legitimate input.
pub const EXTRA_OPTS_MAX_BYTES: usize = 4 * 1024;

/// SEC-33 / TASK-1241: hard upper bound on the number of comma-separated
/// `key=value` pairs in an `extra_opts` fragment. Sized to comfortably
/// admit every option DuckDB's `read_json_auto` recognises today while
/// still bounding resource exposure on the interpolated surface.
pub const EXTRA_OPTS_MAX_PAIRS: usize = 32;

/// Escape a string for safe interpolation into a SQL-standard single-quoted
/// literal.
///
/// SEC-12 (TASK-0729): backslashes are passed through unchanged. DuckDB
/// SQL literals use SQL-standard semantics by default (no `E'…'` prefix);
/// only `'` requires escaping (as `''`). The previous behaviour doubled
/// every `\` to `\\`, which on Windows turned `C:\Users\file.json` into
/// `C:\\Users\\file.json` — a path DuckDB could not open. NULs are still
/// neutralised here (callers go through `sanitize_path_for_sql` first, so
/// no NUL should normally reach this function, but the guard preserves
/// the previous defense in depth).
pub fn escape_sql_string(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\'' => escaped.push_str("''"),
            '\0' => escaped.push_str("\\0"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub fn sanitize_path_for_sql(path: &str) -> String {
    path.replace('\0', "")
}

/// READ-5 / TASK-1002: ASCII-only allowlist. Non-ASCII identifiers are
/// rejected because the SQL-safety contract is over the byte representation
/// of the path, not over Unicode general categories. Letting `is_alphanumeric`
/// (which spans ~140k codepoints across L*/Nd) widen the gate admitted
/// homoglyphs (Cyrillic `а` U+0430), bidi tricks at the rendering layer, and
/// ligatures (`ﬀ` U+FB00) — none of which the downstream `escape_sql_string`
/// neutralises (it only handles `'` and `\\0`). Cross-references SEC-12 /
/// TASK-0729 for the broader interpolated-path threat model. If non-ASCII
/// path support is ever a real requirement, document the allowed scripts
/// explicitly and reject mixed-script identifiers; the current set
/// (`extensions/*`, project / language / file names) is ASCII by policy.
pub fn validate_path_chars(path: &str) -> Result<(), SqlError> {
    // READ-5 (TASK-0528): reject empty paths up front. The character-by-
    // character loop below trivially returns Ok for "", which let
    // forgotten-population bugs slip through and produced
    // `read_json_auto('')` SQL that surfaced as opaque DuckDB errors.
    // Failing fast here keeps the diagnostic close to the offending caller.
    if path.is_empty() {
        return Err(SqlError::EmptyPath);
    }
    for ch in path.chars() {
        let is_safe = ch.is_ascii_alphanumeric()
            || ch == '-'
            || ch == '_'
            || ch == '/'
            || ch == '.'
            || ch == ' '
            // SEC-14: backslash and colon are Windows path metacharacters
            // (`C:\…`, `\\server\share`). On Unix neither has any path
            // meaning — `:` is the PATH-list separator and `\` carries no
            // semantics — so accepting them everywhere weakens defense in
            // depth (e.g. `/tmp/foo:bar` survives validation and lands in
            // logs / future shell contexts where `:` is meaningful).
            // Gate them behind cfg(windows) so each platform sees only the
            // metacharacters it actually needs to handle.
            || (cfg!(windows) && (ch == '\\' || ch == ':'));
        if !is_safe {
            return Err(SqlError::InvalidPathChar(ch));
        }
    }
    Ok(())
}

pub fn validate_no_traversal(path: &Path) -> Result<(), SqlError> {
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(SqlError::PathTraversalNotAllowed(path.to_path_buf()));
        }
    }
    Ok(())
}

/// Combined validate + sanitize + escape for a path destined for SQL interpolation.
///
/// Non-UTF-8 paths are rejected up front (SEC-14) — the previous lossy
/// conversion silently replaced invalid bytes with `U+FFFD`, undermining
/// defense-in-depth.
pub fn prepare_path_for_sql(path: &Path) -> Result<String, SqlError> {
    validate_no_traversal(path)?;
    let path_str = path
        .to_str()
        .ok_or_else(|| SqlError::InvalidUtf8Path(path.as_os_str().to_os_string()))?;
    validate_path_chars(path_str)?;
    let sanitized = sanitize_path_for_sql(path_str);
    Ok(escape_sql_string(&sanitized))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn escape_sql_string_simple() {
        assert_eq!(escape_sql_string("simple"), "simple");
    }

    #[test]
    fn escape_sql_string_quotes() {
        assert_eq!(escape_sql_string("it's"), "it''s");
    }

    /// SEC-12 (TASK-0729): DuckDB SQL literals are SQL-standard (no E''
    /// prefix), so backslashes must pass through unchanged. Doubling them
    /// previously corrupted Windows paths interpolated into DuckDB SQL.
    #[test]
    fn escape_sql_string_backslash_is_preserved() {
        assert_eq!(escape_sql_string(r"path\to\file"), r"path\to\file");
    }

    #[test]
    fn escape_sql_string_null() {
        assert_eq!(escape_sql_string("has\0null"), "has\\0null");
    }

    #[test]
    fn sanitize_path_removes_null() {
        assert_eq!(sanitize_path_for_sql("path\0file"), "pathfile");
    }

    #[test]
    fn validate_path_chars_accepts_safe() {
        assert!(validate_path_chars("/home/user/file.json").is_ok());
        assert!(validate_path_chars("./data-1_file.txt").is_ok());
    }

    #[test]
    #[cfg(windows)]
    fn validate_path_chars_accepts_windows_drive_letter_and_backslash() {
        assert!(validate_path_chars("C:\\Users\\file.json").is_ok());
    }

    /// SEC-14: on Unix, `\\` and `:` carry no path meaning — `:` is the
    /// PATH-list separator and `\` is a shell escape — so they must be
    /// rejected. They are still accepted on Windows where they are part of
    /// legitimate path syntax (`C:\Users\…`, `\\server\share`).
    #[test]
    #[cfg(unix)]
    fn validate_path_chars_rejects_backslash_on_unix() {
        let err = validate_path_chars("/tmp/foo\\bar");
        assert!(matches!(err, Err(SqlError::InvalidPathChar('\\'))));
    }

    #[test]
    #[cfg(unix)]
    fn validate_path_chars_rejects_colon_on_unix() {
        let err = validate_path_chars("/tmp/foo:bar");
        assert!(matches!(err, Err(SqlError::InvalidPathChar(':'))));
    }

    #[test]
    fn validate_path_chars_accepts_spaces() {
        assert!(validate_path_chars("/home/my user/project dir/file.json").is_ok());
    }

    #[test]
    fn validate_path_chars_rejects_semicolon() {
        let err = validate_path_chars("/path;injection");
        assert!(matches!(err, Err(SqlError::InvalidPathChar(';'))));
    }

    #[test]
    fn validate_path_chars_rejects_dollar() {
        let err = validate_path_chars("/path$var");
        assert!(matches!(err, Err(SqlError::InvalidPathChar('$'))));
    }

    #[test]
    fn validate_path_chars_rejects_backtick() {
        let err = validate_path_chars("/path`cmd`");
        assert!(matches!(err, Err(SqlError::InvalidPathChar('`'))));
    }

    #[test]
    fn validate_no_traversal_accepts_normal_path() {
        assert!(validate_no_traversal(&PathBuf::from("/home/user/data.json")).is_ok());
        assert!(validate_no_traversal(&PathBuf::from("./data/file.json")).is_ok());
    }

    #[test]
    fn validate_no_traversal_rejects_parent_dir() {
        let path = PathBuf::from("../../../etc/passwd");
        let err = validate_no_traversal(&path);
        assert!(matches!(err, Err(SqlError::PathTraversalNotAllowed(_))));
    }

    #[test]
    fn validate_no_traversal_rejects_mixed_traversal() {
        let path = PathBuf::from("/home/../etc/passwd");
        let err = validate_no_traversal(&path);
        assert!(matches!(err, Err(SqlError::PathTraversalNotAllowed(_))));
    }

    #[test]
    fn prepare_path_for_sql_valid() {
        let path = PathBuf::from("/home/user/data/file.json");
        let result = prepare_path_for_sql(&path);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "/home/user/data/file.json");
    }

    #[test]
    fn prepare_path_for_sql_rejects_traversal() {
        let path = PathBuf::from("../../../etc/passwd");
        let result = prepare_path_for_sql(&path);
        assert!(result.is_err());
    }

    #[test]
    fn prepare_path_for_sql_rejects_injection() {
        let path = PathBuf::from("/path;DROP TABLE users;");
        let result = prepare_path_for_sql(&path);
        assert!(result.is_err());
    }

    #[test]
    fn prepare_path_for_sql_accepts_spaces() {
        let path = PathBuf::from("/home/my user/project dir/file.json");
        let result = prepare_path_for_sql(&path);
        assert!(result.is_ok());
        assert!(result.unwrap().contains("my user/project dir"));
    }

    // --- validate_identifier tests ---

    #[test]
    fn validate_identifier_accepts_simple_name() {
        assert!(validate_identifier("tokei_files").is_ok());
        assert!(validate_identifier("CrateDeps").is_ok());
        assert!(validate_identifier("_private").is_ok());
        assert!(validate_identifier("t").is_ok());
    }

    #[test]
    fn validate_identifier_rejects_empty() {
        assert!(matches!(
            validate_identifier(""),
            Err(SqlError::InvalidIdentifier(_))
        ));
    }

    #[test]
    fn validate_identifier_rejects_leading_digit() {
        assert!(matches!(
            validate_identifier("1table"),
            Err(SqlError::InvalidIdentifier(_))
        ));
    }

    #[test]
    fn validate_identifier_rejects_sql_injection_semicolon() {
        assert!(validate_identifier("table; DROP TABLE users").is_err());
    }

    #[test]
    fn validate_identifier_rejects_sql_comment_injection() {
        assert!(validate_identifier("table--comment").is_err());
    }

    #[test]
    fn validate_identifier_rejects_union_injection() {
        assert!(validate_identifier("t UNION SELECT * FROM secrets").is_err());
    }

    #[test]
    fn validate_identifier_rejects_quotes() {
        assert!(validate_identifier("table'name").is_err());
        assert!(validate_identifier("table\"name").is_err());
    }

    #[test]
    fn validate_identifier_rejects_dot() {
        assert!(validate_identifier("schema.table").is_err());
    }

    #[test]
    fn validate_identifier_rejects_parentheses() {
        assert!(validate_identifier("name()").is_err());
    }

    // --- escape_sql_string edge cases ---

    #[test]
    fn escape_sql_string_multiple_quotes() {
        assert_eq!(escape_sql_string("it''s"), "it''''s");
    }

    #[test]
    fn escape_sql_string_empty() {
        assert_eq!(escape_sql_string(""), "");
    }

    #[test]
    fn escape_sql_string_unicode_preserved() {
        assert_eq!(escape_sql_string("日本語"), "日本語");
        assert_eq!(escape_sql_string("café"), "café");
        assert_eq!(escape_sql_string("🦀"), "🦀");
    }

    #[test]
    fn escape_sql_string_mixed_dangerous() {
        // SEC-12 (TASK-0729): `'` is doubled, NUL is neutralised, `\` is
        // preserved verbatim (SQL-standard literal semantics).
        let input = "O'Brien\\path\0end";
        let escaped = escape_sql_string(input);
        assert_eq!(escaped, "O''Brien\\path\\0end");
    }

    /// SEC-12 (TASK-0729): a typical Windows absolute path must round-trip
    /// through `prepare_path_for_sql` without backslash duplication so
    /// DuckDB receives the same path the caller intended to open.
    #[test]
    #[cfg(windows)]
    fn prepare_path_for_sql_preserves_windows_backslashes() {
        let path = PathBuf::from(r"C:\Users\file.json");
        let prepared = prepare_path_for_sql(&path).expect("windows path is safe");
        assert_eq!(prepared, r"C:\Users\file.json");
    }

    // --- validate_path_chars edge cases ---

    #[test]
    fn validate_path_chars_rejects_null_byte() {
        assert!(validate_path_chars("path\0file").is_err());
    }

    #[test]
    fn validate_path_chars_rejects_control_chars() {
        assert!(validate_path_chars("path\x01file").is_err());
        assert!(validate_path_chars("path\x1Ffile").is_err());
        assert!(validate_path_chars("path\x7Ffile").is_err());
    }

    #[test]
    fn validate_path_chars_rejects_unicode_special() {
        // Zero-width space (U+200B)
        assert!(validate_path_chars("path\u{200B}file").is_err());
    }

    #[test]
    fn validate_path_chars_rejects_pipe() {
        assert!(validate_path_chars("path|cmd").is_err());
    }

    #[test]
    fn validate_path_chars_rejects_angle_brackets() {
        assert!(validate_path_chars("path<cmd>").is_err());
    }

    // --- prepare_path_for_sql combined attack vectors ---

    #[test]
    fn prepare_path_for_sql_rejects_quote_injection() {
        let path = PathBuf::from("/path'); DROP TABLE users;--");
        assert!(prepare_path_for_sql(&path).is_err());
    }

    #[test]
    fn prepare_path_for_sql_rejects_backtick_subshell() {
        let path = PathBuf::from("/path/`rm -rf /`");
        assert!(prepare_path_for_sql(&path).is_err());
    }

    #[test]
    fn prepare_path_for_sql_rejects_dollar_expansion() {
        let path = PathBuf::from("/path/${HOME}");
        assert!(prepare_path_for_sql(&path).is_err());
    }

    #[test]
    #[cfg(unix)]
    fn prepare_path_for_sql_rejects_non_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let bytes = b"/home/user/\xff\xfe.json";
        let os = OsStr::from_bytes(bytes);
        let path = std::path::Path::new(os);
        let err = prepare_path_for_sql(path);
        assert!(matches!(err, Err(SqlError::InvalidUtf8Path(_))));
    }

    #[test]
    fn prepare_path_for_sql_handles_very_long_path() {
        let long_segment = "a".repeat(4096);
        let path = PathBuf::from(format!("/home/user/{long_segment}/file.json"));
        // Should succeed — length alone is not a security issue
        assert!(prepare_path_for_sql(&path).is_ok());
    }

    #[test]
    fn validate_identifier_rejects_unicode_lookalike() {
        // Cyrillic 'а' (U+0430) looks like Latin 'a' but is not ASCII
        assert!(validate_identifier("\u{0430}table").is_err());
    }

    /// READ-5 / TASK-1002: non-ASCII alphabetics (CJK, ligatures, homoglyphs)
    /// must be rejected. The previous `is_alphanumeric()` allowlist admitted
    /// the entire Unicode L*/Nd categories, letting Cyrillic `а` (U+0430)
    /// flow through as a different codepoint from ASCII `a`, and ligatures
    /// like `ﬀ` (U+FB00) survive validation despite escaping that the
    /// downstream literal escaper does not handle.
    #[test]
    fn validate_path_chars_rejects_non_ascii_alphabetics() {
        // CJK
        assert!(validate_path_chars("/home/用户/file").is_err());
        // Cyrillic 'а' (U+0430) homoglyph for ASCII 'a'
        assert!(validate_path_chars("/home/\u{0430}/file").is_err());
        // Latin small ligature ff (U+FB00)
        assert!(validate_path_chars("/home/\u{FB00}/file").is_err());
    }

    /// READ-5 (TASK-0528): empty paths are now rejected up front, both at
    /// the leaf validator and through `prepare_path_for_sql`. Previously
    /// the for-loop had zero iterations and produced `Ok(())`, letting an
    /// unpopulated path slip through and surface as a confusing
    /// `read_json_auto('')` failure.
    #[test]
    fn validate_path_chars_empty_is_rejected() {
        assert!(matches!(validate_path_chars(""), Err(SqlError::EmptyPath)));
    }

    #[test]
    fn prepare_path_for_sql_rejects_empty() {
        let path = std::path::PathBuf::new();
        assert!(matches!(
            prepare_path_for_sql(&path),
            Err(SqlError::EmptyPath)
        ));
    }

    /// SEC-33 / TASK-1241 AC #2: an oversize `opts` (above the 4-KiB
    /// whole-string cap) must be rejected even when every byte passes
    /// the per-character allowlist. Without the cap a future dynamic
    /// caller could push a multi-megabyte allowlist-conformant string
    /// straight into the interpolated `read_json_auto(..., {opts})`
    /// fragment.
    #[test]
    fn validate_extra_opts_rejects_oversize_input() {
        // A single key=value pair whose value alone exceeds the cap.
        let value = "v".repeat(EXTRA_OPTS_MAX_BYTES);
        let big = format!("k={value}");
        assert!(
            big.len() > EXTRA_OPTS_MAX_BYTES,
            "test setup must exceed cap"
        );
        let err = validate_extra_opts(&big);
        assert!(matches!(err, Err(SqlError::InvalidExtraOpts(_))));
    }

    /// SEC-33 / TASK-1241 AC #2: an `opts` whose pair count exceeds the
    /// 32-pair cap must be rejected. Each pair stays well under the
    /// per-character body cap, so this test isolates the pair-count
    /// guard from the byte-length guard.
    #[test]
    fn validate_extra_opts_rejects_excess_pair_count() {
        let pairs: Vec<String> = (0..(EXTRA_OPTS_MAX_PAIRS + 1))
            .map(|i| format!("k{i}=v"))
            .collect();
        let big = pairs.join(",");
        assert!(
            big.len() <= EXTRA_OPTS_MAX_BYTES,
            "test setup must isolate pair-count guard from byte-length guard"
        );
        let err = validate_extra_opts(&big);
        assert!(matches!(err, Err(SqlError::InvalidExtraOpts(_))));
    }

    /// SEC-33 / TASK-1241: legitimate inputs (1 pair up to the
    /// `EXTRA_OPTS_MAX_PAIRS` cap, body under `EXTRA_OPTS_MAX_BYTES`)
    /// continue to validate so today's static call sites are unaffected.
    #[test]
    fn validate_extra_opts_accepts_at_cap() {
        let pairs: Vec<String> = (0..EXTRA_OPTS_MAX_PAIRS)
            .map(|i| format!("k{i}=v"))
            .collect();
        let at_cap = pairs.join(",");
        assert!(at_cap.len() <= EXTRA_OPTS_MAX_BYTES);
        assert!(validate_extra_opts(&at_cap).is_ok());
    }

    #[test]
    fn escape_sql_string_combined_null_and_quote() {
        assert_eq!(escape_sql_string("val\0ue's"), "val\\0ue''s");
    }
}
