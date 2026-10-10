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
//!   parameters. Rejects the empty string and control characters; every
//!   other character is legitimate in a path and is accepted.
//! - [`validate_no_traversal`] — for path-like strings whose semantics depend
//!   on staying inside a specific root. Reject `..` segments before relying
//!   on prefix joins or filesystem reads.
//!
//! Bound-parameter values are not at risk of injection, so
//! `validate_path_chars` and `validate_no_traversal` exist for **semantic**
//! correctness only (e.g. preventing traversal-based mismatches). No path is
//! interpolated into SQL, so this module has no string-escaping helpers.

use std::path::Path;
use thiserror::Error;

/// A path or identifier failed shared SQL-validation.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum SqlError {
    /// The path contains a control character.
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

pub(super) const fn is_valid_identifier_const(s: &str) -> bool {
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

/// Validate a path-like string that is passed to SQL as a bound parameter.
///
/// A bound value cannot inject SQL, so this is a semantic check, not an
/// allowlist: any character a filesystem path may legitimately contain
/// (spaces, parentheses, `+`, `@`, non-ASCII text, …) is accepted. Only the
/// empty string and control characters are rejected — neither can name a
/// real workspace member, and a control character would corrupt the log
/// lines and prefix matches the value later flows into.
///
/// # Errors
///
/// [`SqlError::EmptyPath`] if `path` is empty, or
/// [`SqlError::InvalidPathChar`] if it contains a control character.
pub fn validate_path_chars(path: &str) -> Result<(), SqlError> {
    // An empty path would pass the loop below trivially and surface later as
    // an opaque engine error far from the caller that forgot to populate it.
    if path.is_empty() {
        return Err(SqlError::EmptyPath);
    }
    path.chars()
        .find(|ch| ch.is_control())
        .map_or(Ok(()), |ch| Err(SqlError::InvalidPathChar(ch)))
}

/// Reject a path that contains a parent-directory (`..`) component.
///
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
    fn validate_path_chars_accepts_spaces() {
        assert!(validate_path_chars("/home/my user/project dir/file.json").is_ok());
    }

    /// The value is a bound parameter, so shell and SQL metacharacters carry
    /// no meaning and must not turn a real directory name into an error.
    #[test]
    fn validate_path_chars_accepts_punctuation_legal_in_paths() {
        for path in [
            "/home/u/My Project (old)",
            "/home/u/proj+x",
            "/home/u/@scope/ws",
            "/home/u/a;b$c`d|e<f>g",
            "C:\\Users\\file.json",
            "/tmp/foo:bar",
        ] {
            assert!(
                validate_path_chars(path).is_ok(),
                "{path:?} is a legal path and must be accepted"
            );
        }
    }

    #[test]
    fn validate_path_chars_accepts_non_ascii() {
        assert!(validate_path_chars("/home/用户/file").is_ok());
        assert!(validate_path_chars("/home/José/проект").is_ok());
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

    /// C1 controls (U+0080–U+009F) are control characters too.
    #[test]
    fn validate_path_chars_rejects_c1_control_chars() {
        assert!(matches!(
            validate_path_chars("path\u{85}file"),
            Err(SqlError::InvalidPathChar('\u{85}'))
        ));
    }

    /// An empty path has no control character to find, so it needs its own
    /// rejection; otherwise an unpopulated path surfaces as a confusing engine
    /// failure far from its caller.
    #[test]
    fn validate_path_chars_empty_is_rejected() {
        assert!(matches!(validate_path_chars(""), Err(SqlError::EmptyPath)));
    }
}
