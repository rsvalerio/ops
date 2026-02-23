//! SQL utilities for cargo metadata.
//!
//! # Security (SEC-001)
//!
//! This module constructs SQL queries with string interpolation for DuckDB's
//! `read_json_auto()` function. While parameterized queries are preferred, DuckDB
//! requires a string literal for file paths in this context.
//!
//! We employ **defense-in-depth** validation to prevent SQL injection:
//!
//! 1. **validate_no_traversal()**: Blocks `..` path components
//! 2. **validate_path_chars()**: Rejects dangerous characters (`;`, `$`, backticks)
//! 3. **sanitize_path_for_sql()**: Removes null bytes
//! 4. **escape_sql_string()**: Escapes quotes and backslashes
//!
//! These layers ensure that even if one check fails, others provide protection.
//! The path is validated before any SQL is constructed.

use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ViewsError {
    #[error("invalid character in path: {0:?}")]
    InvalidPathChar(char),
    #[error("path traversal not allowed: {0}")]
    PathTraversalNotAllowed(String),
}

fn escape_sql_string(s: &str) -> String {
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

/// SEC-004: Defense-in-depth for SQL string interpolation.
///
/// While DuckDB's `read_json_auto()` requires string interpolation for the path,
/// we employ multiple layers of protection:
///
/// 1. **validate_no_traversal()**: Blocks `..` path components
/// 2. **validate_path_chars()**: Rejects dangerous characters (`;`, `$`, backticks, etc.)
/// 3. **sanitize_path_for_sql()**: Removes null bytes
/// 4. **escape_sql_string()**: Escapes quotes and backslashes
///
/// This layered approach ensures that even if one check fails, others provide protection.
/// The path is validated before any SQL is constructed.
fn sanitize_path_for_sql(path: &str) -> String {
    path.replace('\0', "")
}

fn validate_path_chars(path: &str) -> Result<(), ViewsError> {
    for ch in path.chars() {
        let is_safe = ch.is_alphanumeric()
            || ch == '-'
            || ch == '_'
            || ch == '/'
            || ch == '.'
            || ch == '\\'
            || ch == ':';
        if !is_safe {
            return Err(ViewsError::InvalidPathChar(ch));
        }
    }
    Ok(())
}

fn validate_no_traversal(path: &Path) -> Result<(), ViewsError> {
    let path_str = path.to_string_lossy();
    for component in path.components() {
        if matches!(component, std::path::Component::ParentDir) {
            return Err(ViewsError::PathTraversalNotAllowed(path_str.into_owned()));
        }
    }
    Ok(())
}

pub fn metadata_raw_create_sql(path: &Path) -> Result<String, ViewsError> {
    validate_no_traversal(path)?;

    let path_str = path.to_string_lossy();
    validate_path_chars(&path_str)?;
    let sanitized = sanitize_path_for_sql(&path_str);
    let escaped = escape_sql_string(&sanitized);
    Ok(format!(
        "CREATE OR REPLACE TABLE metadata_raw AS SELECT * FROM read_json_auto('{}', maximum_object_size=67108864)",
        escaped
    ))
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
    fn validate_path_chars_rejects_semicolon() {
        let err = validate_path_chars("/path;injection");
        assert!(matches!(err, Err(ViewsError::InvalidPathChar(';'))));
    }

    #[test]
    fn validate_path_chars_rejects_dollar() {
        let err = validate_path_chars("/path$var");
        assert!(matches!(err, Err(ViewsError::InvalidPathChar('$'))));
    }

    #[test]
    fn validate_path_chars_rejects_backtick() {
        let err = validate_path_chars("/path`cmd`");
        assert!(matches!(err, Err(ViewsError::InvalidPathChar('`'))));
    }

    #[test]
    fn metadata_raw_create_sql_valid_path() {
        let path = PathBuf::from("/home/user/data/metadata.json");
        let result = metadata_raw_create_sql(&path);
        assert!(result.is_ok());
        let sql = result.unwrap();
        assert!(sql.contains("read_json_auto"));
        assert!(sql.contains("/home/user/data/metadata.json"));
    }

    #[test]
    fn metadata_raw_create_sql_rejects_injection() {
        let path = PathBuf::from("/path;DROP TABLE users;");
        let result = metadata_raw_create_sql(&path);
        assert!(result.is_err());
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
        assert!(matches!(err, Err(ViewsError::PathTraversalNotAllowed(_))));
    }

    #[test]
    fn validate_no_traversal_rejects_mixed_traversal() {
        let path = PathBuf::from("/home/../etc/passwd");
        let err = validate_no_traversal(&path);
        assert!(matches!(err, Err(ViewsError::PathTraversalNotAllowed(_))));
    }

    #[test]
    fn metadata_raw_create_sql_rejects_traversal() {
        let path = PathBuf::from("../../../etc/passwd");
        let result = metadata_raw_create_sql(&path);
        assert!(result.is_err());
    }
}
