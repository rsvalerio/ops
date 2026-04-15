//! SQL security validation functions for path and identifier safety.

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SqlError {
    #[error("invalid character in path: {0:?}")]
    InvalidPathChar(char),
    #[error("path traversal not allowed: {0}")]
    PathTraversalNotAllowed(String),
    #[error("invalid SQL identifier: {0:?}")]
    InvalidIdentifier(String),
}

/// Validate that a string is a safe SQL identifier (`[a-zA-Z_][a-zA-Z0-9_]*`).
///
/// Used for table names and other identifiers that must be interpolated into SQL.
/// All current call sites pass `&'static str` literals, but this provides
/// defense-in-depth against future misuse.
pub fn validate_identifier(name: &str) -> Result<(), SqlError> {
    if name.is_empty() {
        return Err(SqlError::InvalidIdentifier(name.to_string()));
    }
    let mut chars = name.chars();
    let first = chars.next().unwrap();
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

pub fn escape_sql_string(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\'' => escaped.push_str("''"),
            '\0' => escaped.push_str("\\0"),
            '\\' => escaped.push_str("\\\\"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub fn sanitize_path_for_sql(path: &str) -> String {
    path.replace('\0', "")
}

pub fn validate_path_chars(path: &str) -> Result<(), SqlError> {
    for ch in path.chars() {
        let is_safe = ch.is_alphanumeric()
            || ch == '-'
            || ch == '_'
            || ch == '/'
            || ch == '.'
            || ch == '\\'
            || ch == ':'
            || ch == ' ';
        if !is_safe {
            return Err(SqlError::InvalidPathChar(ch));
        }
    }
    Ok(())
}

pub fn validate_no_traversal(path: &Path) -> Result<(), SqlError> {
    let path_str = path.to_string_lossy();
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(SqlError::PathTraversalNotAllowed(path_str.into_owned()));
        }
    }
    Ok(())
}

/// Combined validate + sanitize + escape for a path destined for SQL interpolation.
pub fn prepare_path_for_sql(path: &Path) -> Result<String, SqlError> {
    validate_no_traversal(path)?;
    let path_str = path.to_string_lossy();
    validate_path_chars(&path_str)?;
    let sanitized = sanitize_path_for_sql(&path_str);
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

    #[test]
    fn escape_sql_string_backslash() {
        assert_eq!(escape_sql_string(r#"path\to\file"#), r#"path\\to\\file"#);
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
        assert!(validate_path_chars("C:\\Users\\file.json").is_ok());
        assert!(validate_path_chars("./data-1_file.txt").is_ok());
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
        let input = "O'Brien\\path\0end";
        let escaped = escape_sql_string(input);
        assert_eq!(escaped, "O''Brien\\\\path\\0end");
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

    #[test]
    fn validate_path_chars_accepts_cjk() {
        // CJK characters are alphanumeric per Unicode — accepted by path validation
        assert!(validate_path_chars("/home/用户/file").is_ok());
    }

    #[test]
    fn validate_path_chars_empty_is_ok() {
        assert!(validate_path_chars("").is_ok());
    }

    #[test]
    fn escape_sql_string_combined_null_and_quote() {
        assert_eq!(escape_sql_string("val\0ue's"), "val\\0ue''s");
    }
}
