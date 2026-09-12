//! SQL security validation functions for path and identifier safety.
//!
//! ARCH-9 / TASK-1862: this module is `pub(crate)`. Only the items
//! re-exported from [`crate::sql`] (`SqlError`, `TableName`, `quoted_ident`)
//! cross the crate boundary; the granular validators are reachable inside
//! the crate only, so a downstream caller cannot reach for a single helper
//! and opt out of the defence-in-depth stack below.
//!
//! # Helper composition
//!
//! Each helper guards a different threat surface; many sites need more than one.
//!
//! - [`validate_identifier`] / [`quoted_ident`] — for any identifier interpolated
//!   into SQL (table, column, view names). `quoted_ident` is preferred at call
//!   sites because it cannot be invoked without validation.
//! - [`validate_path_chars`] — for path-like strings used as bound
//!   parameters. Catches dangerous shell/SQL metacharacters and control
//!   codes.
//! - [`validate_no_traversal`] — for path-like strings whose semantics depend
//!   on staying inside a specific root. Reject `..` segments before relying
//!   on `starts_with` joins or filesystem reads.
//!
//! Bound-parameter values still benefit from `validate_path_chars` and
//! `validate_no_traversal` for **semantic** correctness (e.g., preventing
//! traversal-based mismatches), even though they are not at risk of
//! injection.
//!
//! SQLite port note: the string-escaping helpers (`escape_sql_string`,
//! `sanitize_path_for_sql`, `prepare_path_for_sql`) and the `ExtraOpts`
//! gate died with `read_json_auto` — no path or option fragment is
//! interpolated into SQL anymore, so there is nothing to escape.

use std::path::Path;
use thiserror::Error;

/// A path or identifier failed shared SQL-validation.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SqlError {
    /// The path contains a character outside the accepted set.
    #[error("invalid character in path: {0:?}")]
    InvalidPathChar(char),
    /// The path escapes its intended directory via `..` or an absolute
    /// prefix.
    #[error("path traversal not allowed: {}", .0.display())]
    PathTraversalNotAllowed(std::path::PathBuf),
    /// The identifier does not match the `[a-zA-Z_][a-zA-Z0-9_]*` shape.
    #[error("invalid SQL identifier: {0:?}")]
    InvalidIdentifier(String),
    /// The path is not valid UTF-8.
    #[error("path is not valid UTF-8: {0:?}")]
    InvalidUtf8Path(std::ffi::OsString),
    /// The path is the empty string.
    #[error("path is empty")]
    EmptyPath,
}

/// Validate that a string is a safe SQL identifier (`[a-zA-Z_][a-zA-Z0-9_]*`).
///
/// Used for table names and other identifiers that must be interpolated into SQL.
/// All current call sites pass `&'static str` literals, but this provides
/// defense-in-depth against future misuse.
///
/// # Errors
///
/// [`SqlError::InvalidIdentifier`] if `name` is empty, starts with anything
/// other than an ASCII letter or `_`, or contains a character outside
/// `[A-Za-z0-9_]`.
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
///
/// READ-1 / TASK-1624: not to be confused with
/// `crate::sql::query::helpers::QueryTableName`, which is a *runtime*-
/// validated equivalent used by the per-crate query scaffolding. Both
/// share the SEC-12 "identifier allowlist before interpolation" contract
/// but differ in when validation runs (build-time vs. runtime).
#[derive(Debug, Clone, Copy)]
pub struct TableName(&'static str);

impl TableName {
    /// Const-validating constructor: panics at compile time if `s` is not
    /// a valid SQL identifier (`[A-Za-z_][A-Za-z0-9_]*`). Designed to be
    /// called from `const fn` constructors so the static-table-name
    /// invariant is enforced at build time.
    ///
    /// # Panics
    ///
    /// If `s` is not a valid SQL identifier. In a `const` context this is a
    /// compile-time error; at runtime it aborts the process.
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
    // Slice patterns walk the bytes without any indexing, so the empty and
    // out-of-bounds cases are handled by construction.
    let (first, mut rest) = match s.as_bytes() {
        [] => return false,
        [first, rest @ ..] => (*first, rest),
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    while let [b, tail @ ..] = rest {
        if !(b.is_ascii_alphanumeric() || *b == b'_') {
            return false;
        }
        rest = tail;
    }
    true
}

/// Validate `name` and return a double-quoted SQL identifier in one step.
///
/// Use this helper at every site that interpolates a table or column name into
/// a SQL string — it guarantees the identifier is validated before quoting,
/// closing off forgotten-validation regressions.
///
/// # Errors
///
/// [`SqlError::InvalidIdentifier`] if `name` fails `validate_identifier`.
pub fn quoted_ident(name: &str) -> Result<String, SqlError> {
    validate_identifier(name)?;
    Ok(format!("\"{name}\""))
}

/// READ-5 / TASK-1002: ASCII-only allowlist.
///
/// Non-ASCII identifiers are rejected because the SQL-safety contract is over
/// the byte representation of the path, not over Unicode general categories.
/// Letting `is_alphanumeric` (which spans ~140k codepoints across L*/Nd) widen
/// the gate admitted homoglyphs (Cyrillic `а` U+0430), bidi tricks at the
/// rendering layer, and ligatures (`ﬀ` U+FB00). If non-ASCII path support is
/// ever a real requirement, document the allowed scripts explicitly and reject
/// mixed-script identifiers; the current set (`extensions/*`, project /
/// language / file names) is ASCII by policy.
///
/// # Errors
///
/// [`SqlError::EmptyPath`] if `path` is empty, or
/// [`SqlError::InvalidPathChar`] if it contains a character outside the
/// safe set (ASCII alphanumerics, `-`, `_`, `/`, `.`, and the platform
/// separator).
pub fn validate_path_chars(path: &str) -> Result<(), SqlError> {
    // READ-5 (TASK-0528): reject empty paths up front. The character-by-
    // character loop below trivially returns Ok for "", which let
    // forgotten-population bugs slip through and surfaced as opaque engine
    // errors far from the caller. Failing fast here keeps the diagnostic
    // close to the offending caller.
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

/// # Errors
///
/// [`SqlError::PathTraversalNotAllowed`] if any component of `path` is `..`.
pub fn validate_no_traversal(path: &Path) -> Result<(), SqlError> {
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(SqlError::PathTraversalNotAllowed(path.to_path_buf()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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

    #[test]
    fn validate_identifier_rejects_unicode_lookalike() {
        // Cyrillic 'а' (U+0430) looks like Latin 'a' but is not ASCII
        assert!(validate_identifier("\u{0430}table").is_err());
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

    /// READ-5 / TASK-1002: non-ASCII alphabetics (CJK, ligatures, homoglyphs)
    /// must be rejected. The previous `is_alphanumeric()` allowlist admitted
    /// the entire Unicode L*/Nd categories, letting Cyrillic `а` (U+0430)
    /// flow through as a different codepoint from ASCII `a`, and ligatures
    /// like `ﬀ` (U+FB00) survive validation.
    #[test]
    fn validate_path_chars_rejects_non_ascii_alphabetics() {
        // CJK
        assert!(validate_path_chars("/home/用户/file").is_err());
        // Cyrillic 'а' (U+0430) homoglyph for ASCII 'a'
        assert!(validate_path_chars("/home/\u{0430}/file").is_err());
        // Latin small ligature ff (U+FB00)
        assert!(validate_path_chars("/home/\u{FB00}/file").is_err());
    }

    /// READ-5 (TASK-0528): empty paths are rejected up front. The for-loop
    /// has zero iterations and would otherwise return `Ok(())`, letting an
    /// unpopulated path slip through and surface as a confusing engine
    /// failure far from its caller.
    #[test]
    fn validate_path_chars_empty_is_rejected() {
        assert!(matches!(validate_path_chars(""), Err(SqlError::EmptyPath)));
    }
}
