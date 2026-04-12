//! Shared SQL utilities for DuckDB extensions.
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

use crate::{DbError, DbResult, DuckDb};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SqlError {
    #[error("invalid character in path: {0:?}")]
    InvalidPathChar(char),
    #[error("path traversal not allowed: {0}")]
    PathTraversalNotAllowed(String),
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

/// Generate `CREATE OR REPLACE TABLE <name> AS SELECT * FROM read_json_auto(...)` SQL (DUP-009).
///
/// Validates and escapes the path for safe interpolation. Pass `extra_opts` for
/// additional read_json_auto parameters (e.g., `"maximum_object_size=67108864"`).
pub fn create_table_from_json_sql(
    table_name: &str,
    path: &Path,
    extra_opts: Option<&str>,
) -> Result<String, SqlError> {
    let escaped = prepare_path_for_sql(path)?;
    match extra_opts {
        Some(opts) => Ok(format!(
            "CREATE OR REPLACE TABLE {} AS SELECT * FROM read_json_auto('{}', {})",
            table_name, escaped, opts
        )),
        None => Ok(format!(
            "CREATE OR REPLACE TABLE {} AS SELECT * FROM read_json_auto('{}')",
            table_name, escaped
        )),
    }
}

/// Check if a table or view exists in the database.
fn table_exists(conn: &duckdb::Connection, table_name: &str) -> Result<bool, anyhow::Error> {
    use anyhow::Context;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM information_schema.tables WHERE table_name = ?",
            [table_name],
            |row: &duckdb::Row| row.get(0),
        )
        .with_context(|| format!("checking if {} exists", table_name))?;
    Ok(count > 0)
}

/// Check if a table exists and has at least one row.
pub fn table_has_data(db: &DuckDb, table_name: &str) -> Result<bool, anyhow::Error> {
    use anyhow::Context;

    let conn = db.lock().context("acquiring db lock")?;
    if !table_exists(&conn, table_name)? {
        return Ok(false);
    }
    // table_name needs interpolation for the COUNT query since DuckDB
    // doesn't support parameterized table names.
    validate_path_chars(table_name)?;
    let escaped = escape_sql_string(table_name);
    let row_count: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM \"{}\"", escaped),
            [],
            |row: &duckdb::Row| row.get(0),
        )
        .with_context(|| format!("counting rows in {}", table_name))?;
    Ok(row_count > 0)
}

/// Compute the ingest data directory from a DB path (appends `.ingest`).
pub fn data_dir_for_db(db_path: &Path) -> PathBuf {
    let mut path = db_path.as_os_str().to_os_string();
    path.push(".ingest");
    PathBuf::from(path)
}

/// Default DB path for a workspace root (using default DataConfig).
pub fn default_db_path(workspace_root: &Path) -> PathBuf {
    DuckDb::resolve_path(&ops_core::config::DataConfig::default(), workspace_root)
}

/// Default data directory for a workspace root.
#[allow(dead_code)]
pub fn default_data_dir(workspace_root: &Path) -> PathBuf {
    data_dir_for_db(&default_db_path(workspace_root))
}

/// Convert an error into a DbError::Io (for wrapping non-IO errors).
pub fn io_err<E: Into<Box<dyn std::error::Error + Send + Sync>>>(e: E) -> DbError {
    DbError::Io(std::io::Error::other(e))
}

/// Compute SHA-256 checksum of a file, returning hex string.
pub fn checksum_file(path: &Path) -> DbResult<String> {
    use sha2::{Digest, Sha256};
    let data = std::fs::read(path).map_err(DbError::Io)?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let digest = hasher.finalize();
    Ok(hex::encode(digest.as_ref() as &[u8]))
}

/// Write a workspace root sidecar file alongside collected data.
///
/// Used by ingestors that don't embed workspace_root in their JSON output
/// (e.g., tokei, coverage). The sidecar is read back during `load()` for
/// `upsert_data_source`.
pub fn write_workspace_sidecar(
    data_dir: &Path,
    name: &str,
    working_directory: &Path,
) -> DbResult<()> {
    let workspace_path = data_dir.join(format!("{}_workspace.txt", name));
    std::fs::write(
        &workspace_path,
        working_directory.to_string_lossy().as_bytes(),
    )
    .map_err(DbError::Io)
}

/// Read a workspace root sidecar file written during collect.
pub fn read_workspace_sidecar(data_dir: &Path, name: &str) -> DbResult<String> {
    let workspace_path = data_dir.join(format!("{}_workspace.txt", name));
    std::fs::read_to_string(&workspace_path).map_err(DbError::Io)
}

/// Remove a workspace root sidecar file (best-effort, ignores errors).
pub fn remove_workspace_sidecar(data_dir: &Path, name: &str) {
    let workspace_path = data_dir.join(format!("{}_workspace.txt", name));
    let _ = std::fs::remove_file(&workspace_path);
}

/// DUP-031: Generic helper to query rows from DuckDB and return as a JSON array.
///
/// Acquires the connection lock, prepares `sql`, maps each row via `row_mapper`,
/// and collects into `Value::Array`.
pub fn query_rows_to_json<F>(
    db: &DuckDb,
    sql: &str,
    row_mapper: F,
) -> Result<serde_json::Value, anyhow::Error>
where
    F: Fn(&duckdb::Row<'_>) -> Result<serde_json::Value, duckdb::Error>,
{
    use anyhow::Context;
    let conn = db.lock().context("acquiring db lock for query")?;
    let mut stmt = conn.prepare(sql).context("preparing query")?;
    let rows = stmt
        .query_map([], |row| row_mapper(row))
        .context("querying")?;
    let mut results = Vec::new();
    for row in rows {
        results.push(row.context("reading row")?);
    }
    Ok(serde_json::Value::Array(results))
}

/// DUP-028/029/030: Refresh an ingestor (collect + load) and return query results.
///
/// Orchestrates the full pipeline: check if table has data, if not collect and load,
/// then query. Used by `provide_from_db` implementations.
///
/// When `ctx.refresh` is true, drops existing data so it will be re-collected.
pub fn provide_via_ingestor<I, Q>(
    db: &DuckDb,
    ctx: &ops_extension::Context,
    table_name: &str,
    ingestor: &I,
    query_fn: Q,
) -> Result<serde_json::Value, anyhow::Error>
where
    I: crate::DataIngestor,
    Q: FnOnce(&DuckDb) -> Result<serde_json::Value, anyhow::Error>,
{
    if ctx.refresh {
        drop_table_if_exists(db, table_name)?;
    }
    if !table_has_data(db, table_name)? {
        let data_dir = data_dir_for_db(db.path());
        std::fs::create_dir_all(&data_dir).map_err(DbError::Io)?;
        ingestor.collect(ctx, &data_dir)?;
        crate::init_schema(db)?;
        ingestor.load(&data_dir, db)?;
    }
    query_fn(db)
}

/// Drop a table if it exists (used by refresh to force re-collection).
fn drop_table_if_exists(db: &DuckDb, table_name: &str) -> Result<(), anyhow::Error> {
    use anyhow::Context;
    validate_path_chars(table_name)?;
    let escaped = escape_sql_string(table_name);
    let conn = db.lock().context("acquiring db lock for drop")?;
    conn.execute_batch(&format!("DROP TABLE IF EXISTS \"{}\"", escaped))
        .with_context(|| format!("dropping table {}", table_name))?;
    Ok(())
}

/// DUP-032: Macro to generate standard path validation tests for `*_create_sql` functions.
///
/// Generates four tests: valid path, path with spaces, injection rejection, traversal rejection.
#[macro_export]
macro_rules! test_create_sql_validation {
    ($create_fn:path, $file_name:expr) => {
        #[test]
        fn create_sql_valid_path() {
            let path = std::path::PathBuf::from(concat!("/home/user/data/", $file_name));
            let result = $create_fn(&path);
            assert!(result.is_ok());
            let sql = result.unwrap();
            assert!(sql.contains("read_json_auto"));
            assert!(sql.contains($file_name));
        }

        #[test]
        fn create_sql_accepts_path_with_spaces() {
            let path = std::path::PathBuf::from(concat!("/home/my user/project dir/", $file_name));
            let result = $create_fn(&path);
            assert!(result.is_ok());
            assert!(result.unwrap().contains("my user/project dir"));
        }

        #[test]
        fn create_sql_rejects_injection() {
            let path = std::path::PathBuf::from("/path;DROP TABLE users;");
            let result = $create_fn(&path);
            assert!(result.is_err());
        }

        #[test]
        fn create_sql_rejects_traversal() {
            let path = std::path::PathBuf::from("../../../etc/passwd");
            let result = $create_fn(&path);
            assert!(result.is_err());
        }
    };
}

/// Query total file count across the whole project from `tokei_files`.
pub fn query_project_file_count(db: &DuckDb) -> anyhow::Result<i64> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_project_file_count")?;

    if !table_exists(&conn, "tokei_files")? {
        return Ok(0);
    }

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM tokei_files",
            [],
            |row: &duckdb::Row| row.get(0),
        )
        .context("querying project file count")?;

    Ok(count)
}

/// Query per-crate file counts from `tokei_files`.
///
/// Returns a map of member path -> file count. Members with no matching
/// files get 0.
pub fn query_crate_file_count(
    db: &DuckDb,
    member_paths: &[&str],
) -> anyhow::Result<HashMap<String, i64>> {
    use anyhow::Context;

    if member_paths.is_empty() {
        return Ok(HashMap::new());
    }

    for path in member_paths {
        validate_path_chars(path)?;
    }

    let conn = db
        .lock()
        .context("acquiring db lock for query_crate_file_count")?;

    if !table_exists(&conn, "tokei_files")? {
        return Ok(member_paths.iter().map(|p| (p.to_string(), 0)).collect());
    }

    let values: Vec<String> = member_paths
        .iter()
        .map(|p| format!("('{}')", escape_sql_string(p)))
        .collect();

    let sql = format!(
        "WITH members(path) AS (VALUES {}) \
         SELECT m.path, COUNT(f.file) AS files \
         FROM members m \
         LEFT JOIN tokei_files f ON starts_with(f.file, m.path || '/') \
         GROUP BY m.path",
        values.join(", ")
    );

    let mut stmt = conn
        .prepare(&sql)
        .context("preparing query_crate_file_count")?;
    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .context("querying crate file counts")?;

    let mut result = HashMap::new();
    for row in rows {
        let (path, count) = row.context("reading crate file count row")?;
        result.insert(path, count);
    }
    Ok(result)
}

/// Query total lines of code across the whole project from `tokei_files`.
pub fn query_project_loc(db: &DuckDb) -> anyhow::Result<i64> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_project_loc")?;

    if !table_exists(&conn, "tokei_files")? {
        return Ok(0);
    }

    let loc: i64 = conn
        .query_row(
            "SELECT COALESCE(SUM(code), 0) FROM tokei_files",
            [],
            |row: &duckdb::Row| row.get(0),
        )
        .context("querying project LOC")?;

    Ok(loc)
}

/// Query total dependency count from `crate_dependencies`.
pub fn query_dependency_count(db: &DuckDb) -> anyhow::Result<usize> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_dependency_count")?;

    if !table_exists(&conn, "crate_dependencies")? {
        return Ok(0);
    }

    let count: i64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT dependency_name) FROM crate_dependencies",
            [],
            |row: &duckdb::Row| row.get(0),
        )
        .context("querying dependency count")?;

    Ok(count as usize)
}

/// Query distinct languages from `tokei_files` with LOC percentage, ordered by total LOC descending.
///
/// Returns formatted strings like `"Rust 85.2%"`. Languages under 0.1% are omitted.
pub fn query_project_languages(db: &DuckDb) -> anyhow::Result<Vec<String>> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_project_languages")?;

    if !table_exists(&conn, "tokei_files")? {
        return Ok(vec![]);
    }

    let mut stmt = conn
        .prepare(
            "SELECT language, \
                    ROUND(SUM(code) * 100.0 / NULLIF((SELECT SUM(code) FROM tokei_files), 0), 1) AS pct \
             FROM tokei_files \
             GROUP BY language \
             ORDER BY SUM(code) DESC",
        )
        .context("preparing query_project_languages")?;

    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            let lang: String = row.get(0)?;
            let pct: f64 = row.get(1)?;
            Ok((lang, pct))
        })
        .context("querying project languages")?;

    let mut languages = Vec::new();
    for row in rows {
        let (lang, pct) = row.context("reading language row")?;
        if pct >= 0.1 {
            languages.push(format!("{lang} {pct:.1}%"));
        }
    }
    Ok(languages)
}

/// Query per-crate lines of code from `tokei_files`.
///
/// Returns a map of member path -> total code lines. Members with no matching
/// files get 0.
pub fn query_crate_loc(db: &DuckDb, member_paths: &[&str]) -> anyhow::Result<HashMap<String, i64>> {
    use anyhow::Context;

    if member_paths.is_empty() {
        return Ok(HashMap::new());
    }

    // Validate all paths before building SQL
    for path in member_paths {
        validate_path_chars(path)?;
    }

    let conn = db.lock().context("acquiring db lock for query_crate_loc")?;

    if !table_exists(&conn, "tokei_files")? {
        return Ok(member_paths.iter().map(|p| (p.to_string(), 0)).collect());
    }

    // Build VALUES list for CTE
    let values: Vec<String> = member_paths
        .iter()
        .map(|p| format!("('{}')", escape_sql_string(p)))
        .collect();

    let sql = format!(
        "WITH members(path) AS (VALUES {}) \
         SELECT m.path, COALESCE(SUM(f.code), 0) AS loc \
         FROM members m \
         LEFT JOIN tokei_files f ON starts_with(f.file, m.path || '/') \
         GROUP BY m.path",
        values.join(", ")
    );

    let mut stmt = conn.prepare(&sql).context("preparing query_crate_loc")?;
    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .context("querying crate LOC")?;

    let mut result = HashMap::new();
    for row in rows {
        let (path, loc) = row.context("reading crate LOC row")?;
        result.insert(path, loc);
    }
    Ok(result)
}

/// Per-crate coverage data from `coverage_files`.
#[derive(Debug, Clone)]
pub struct CrateCoverage {
    pub lines_count: i64,
    pub lines_covered: i64,
    pub lines_percent: f64,
}

/// Query total coverage across the whole project from `coverage_files`.
pub fn query_project_coverage(db: &DuckDb) -> anyhow::Result<CrateCoverage> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_project_coverage")?;

    if !table_exists(&conn, "coverage_files")? {
        return Ok(CrateCoverage {
            lines_count: 0,
            lines_covered: 0,
            lines_percent: 0.0,
        });
    }

    conn.query_row(
        "SELECT COALESCE(SUM(lines_count), 0), \
                COALESCE(SUM(lines_covered), 0), \
                CASE WHEN SUM(lines_count) > 0 \
                    THEN ROUND(SUM(lines_covered) * 100.0 / SUM(lines_count), 2) \
                    ELSE 0.0 END \
         FROM coverage_files",
        [],
        |row: &duckdb::Row| {
            Ok(CrateCoverage {
                lines_count: row.get(0)?,
                lines_covered: row.get(1)?,
                lines_percent: row.get(2)?,
            })
        },
    )
    .context("querying project coverage")
}

/// Query per-crate coverage from `coverage_files`.
///
/// Returns a map of member path -> CrateCoverage. Members with no matching
/// files get zeroed coverage. Handles both absolute and relative filenames
/// from LLVM coverage output.
pub fn query_crate_coverage(
    db: &DuckDb,
    member_paths: &[&str],
    workspace_root: &str,
) -> anyhow::Result<HashMap<String, CrateCoverage>> {
    use anyhow::Context;

    if member_paths.is_empty() {
        return Ok(HashMap::new());
    }

    // Validate all paths before building SQL
    for path in member_paths {
        validate_path_chars(path)?;
    }
    validate_path_chars(workspace_root)?;

    let conn = db
        .lock()
        .context("acquiring db lock for query_crate_coverage")?;

    if !table_exists(&conn, "coverage_files")? {
        return Ok(member_paths
            .iter()
            .map(|p| {
                (
                    p.to_string(),
                    CrateCoverage {
                        lines_count: 0,
                        lines_covered: 0,
                        lines_percent: 0.0,
                    },
                )
            })
            .collect());
    }

    let values: Vec<String> = member_paths
        .iter()
        .map(|p| format!("('{}')", escape_sql_string(p)))
        .collect();

    let escaped_root = escape_sql_string(workspace_root);

    let sql = format!(
        "WITH members(path) AS (VALUES {}) \
         SELECT m.path, \
                COALESCE(SUM(c.lines_count), 0), \
                COALESCE(SUM(c.lines_covered), 0), \
                CASE WHEN SUM(c.lines_count) > 0 \
                    THEN ROUND(SUM(c.lines_covered) * 100.0 / SUM(c.lines_count), 2) \
                    ELSE 0.0 END \
         FROM members m \
         LEFT JOIN coverage_files c \
             ON starts_with(c.filename, m.path || '/') \
             OR starts_with(c.filename, '{}' || '/' || m.path || '/') \
         GROUP BY m.path",
        values.join(", "),
        escaped_root
    );

    let mut stmt = conn
        .prepare(&sql)
        .context("preparing query_crate_coverage")?;
    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            Ok((
                row.get::<_, String>(0)?,
                CrateCoverage {
                    lines_count: row.get(1)?,
                    lines_covered: row.get(2)?,
                    lines_percent: row.get(3)?,
                },
            ))
        })
        .context("querying crate coverage")?;

    let mut result = HashMap::new();
    for row in rows {
        let (path, cov) = row.context("reading crate coverage row")?;
        result.insert(path, cov);
    }
    Ok(result)
}

/// Query per-crate external dependencies (name + version_req) from `crate_dependencies` view.
///
/// Returns a map of crate_name → Vec<(dep_name, version_req)>, sorted by dep name.
/// Returns an empty map if the view doesn't exist (graceful degradation).
pub fn query_crate_deps(db: &DuckDb) -> anyhow::Result<HashMap<String, Vec<(String, String)>>> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_crate_deps")?;

    if !table_exists(&conn, "crate_dependencies")? {
        return Ok(HashMap::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT crate_name, dependency_name, version_req \
             FROM crate_dependencies \
             WHERE dependency_kind = 'normal' \
             ORDER BY crate_name, dependency_name",
        )
        .context("preparing query_crate_deps")?;

    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .context("querying crate deps")?;

    let mut result: HashMap<String, Vec<(String, String)>> = HashMap::new();
    for row in rows {
        let (crate_name, dep_name, version_req) = row.context("reading crate dep row")?;
        result
            .entry(crate_name)
            .or_default()
            .push((dep_name, version_req));
    }
    Ok(result)
}

/// Query per-crate external dependency counts from `crate_dependencies` view.
///
/// Returns a map of package name → normal dependency count.
/// Returns an empty map if the view doesn't exist (graceful degradation).
pub fn query_crate_dep_counts(db: &DuckDb) -> anyhow::Result<HashMap<String, i64>> {
    use anyhow::Context;

    let conn = db
        .lock()
        .context("acquiring db lock for query_crate_dep_counts")?;

    if !table_exists(&conn, "crate_dependencies")? {
        return Ok(HashMap::new());
    }

    let mut stmt = conn
        .prepare(
            "SELECT crate_name, COUNT(*) AS dep_count \
             FROM crate_dependencies \
             WHERE dependency_kind = 'normal' \
             GROUP BY crate_name",
        )
        .context("preparing query_crate_dep_counts")?;

    let rows = stmt
        .query_map([], |row: &duckdb::Row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .context("querying crate dep counts")?;

    let mut result = HashMap::new();
    for row in rows {
        let (name, count) = row.context("reading crate dep count row")?;
        result.insert(name, count);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_schema;
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

    #[test]
    fn table_has_data_no_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        let result = table_has_data(&db, "nonexistent_table").expect("should succeed");
        assert!(!result);
    }

    #[test]
    fn table_has_data_empty_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        let conn = db.lock().expect("lock");
        conn.execute_batch("CREATE TABLE test_table (id INTEGER)")
            .expect("create table");
        drop(conn);
        let result = table_has_data(&db, "test_table").expect("should succeed");
        assert!(!result);
    }

    #[test]
    fn table_has_data_with_rows() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE test_table (id INTEGER); INSERT INTO test_table VALUES (1)",
        )
        .expect("create and insert");
        drop(conn);
        let result = table_has_data(&db, "test_table").expect("should succeed");
        assert!(result);
    }

    #[test]
    fn data_dir_for_db_appends_ingest() {
        let path = PathBuf::from("/home/proj/target/ops/data.duckdb");
        let result = data_dir_for_db(&path);
        assert_eq!(
            result,
            PathBuf::from("/home/proj/target/ops/data.duckdb.ingest")
        );
    }

    #[test]
    fn default_db_path_uses_target_dir() {
        let root = PathBuf::from("/home/proj");
        let path = default_db_path(&root);
        assert_eq!(path, PathBuf::from("/home/proj/target/ops/data.duckdb"));
    }

    #[test]
    fn io_err_wraps_display_error() {
        let err = io_err("test error message");
        let msg = err.to_string();
        assert!(msg.contains("test error message"));
    }

    #[test]
    fn checksum_file_returns_sha256_hex() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.json");
        std::fs::write(&path, r#"{"test": "data"}"#).expect("write");
        let checksum = checksum_file(&path).expect("checksum");
        assert_eq!(checksum.len(), 64, "SHA-256 hex should be 64 chars");
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn checksum_file_fails_when_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = checksum_file(&dir.path().join("nonexistent.json"));
        assert!(result.is_err(), "should fail for missing file");
    }

    #[test]
    fn checksum_file_is_deterministic() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("test.json");
        std::fs::write(&path, b"test data").expect("write");
        let c1 = checksum_file(&path).expect("checksum1");
        let c2 = checksum_file(&path).expect("checksum2");
        assert_eq!(c1, c2, "checksum should be deterministic");
    }

    #[test]
    fn query_crate_loc_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
             comments BIGINT, blanks BIGINT, lines BIGINT);
             INSERT INTO tokei_files VALUES ('Rust', 'crates/my-lib/src/lib.rs', 3000, 200, 100, 3300);
             INSERT INTO tokei_files VALUES ('Rust', 'crates/my-lib/src/utils.rs', 1231, 50, 30, 1311);
             INSERT INTO tokei_files VALUES ('Rust', 'crates/my-cli/src/main.rs', 1892, 100, 50, 2042);",
        )
        .expect("insert test data");
        drop(conn);

        let result =
            query_crate_loc(&db, &["crates/my-lib", "crates/my-cli"]).expect("query should work");
        assert_eq!(result.len(), 2);
        assert_eq!(result["crates/my-lib"], 4231);
        assert_eq!(result["crates/my-cli"], 1892);
    }

    #[test]
    fn query_crate_loc_empty_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
             comments BIGINT, blanks BIGINT, lines BIGINT);",
        )
        .expect("create empty table");
        drop(conn);

        let result =
            query_crate_loc(&db, &["crates/my-lib", "crates/my-cli"]).expect("query should work");
        assert_eq!(result["crates/my-lib"], 0);
        assert_eq!(result["crates/my-cli"], 0);
    }

    #[test]
    fn query_crate_loc_no_members() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let result = query_crate_loc(&db, &[]).expect("query should work");
        assert!(result.is_empty());
    }

    #[test]
    fn query_project_loc_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
             comments BIGINT, blanks BIGINT, lines BIGINT);
             INSERT INTO tokei_files VALUES ('Rust', 'src/main.rs', 500, 50, 20, 570);
             INSERT INTO tokei_files VALUES ('Rust', 'src/lib.rs', 300, 30, 10, 340);
             INSERT INTO tokei_files VALUES ('TOML', 'Cargo.toml', 40, 5, 3, 48);",
        )
        .expect("insert test data");
        drop(conn);

        let loc = query_project_loc(&db).expect("query should work");
        assert_eq!(loc, 840);
    }

    #[test]
    fn query_crate_deps_no_view() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        let result = query_crate_deps(&db).expect("query should work");
        assert!(result.is_empty());
    }

    #[test]
    fn query_crate_deps_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE VIEW crate_dependencies AS \
             SELECT * FROM (VALUES \
                 ('ops-core', 'anyhow', '^1.0', 'normal', false), \
                 ('ops-core', 'serde', '^1.0', 'normal', false), \
                 ('ops-core', 'tempfile', '^3.0', 'dev', false), \
                 ('ops-cli', 'clap', '^4.0', 'normal', false), \
                 ('ops-cli', 'tokio', '^1.0', 'normal', false) \
             ) AS t(crate_name, dependency_name, version_req, dependency_kind, is_optional)",
        )
        .expect("create view with test data");
        drop(conn);

        let result = query_crate_deps(&db).expect("query should work");
        assert_eq!(result.len(), 2);

        let core_deps = &result["ops-core"];
        assert_eq!(core_deps.len(), 2); // only normal deps
        assert_eq!(core_deps[0], ("anyhow".to_string(), "^1.0".to_string()));
        assert_eq!(core_deps[1], ("serde".to_string(), "^1.0".to_string()));

        let cli_deps = &result["ops-cli"];
        assert_eq!(cli_deps.len(), 2);
        assert_eq!(cli_deps[0], ("clap".to_string(), "^4.0".to_string()));
        assert_eq!(cli_deps[1], ("tokio".to_string(), "^1.0".to_string()));
    }

    #[test]
    fn query_crate_dep_counts_no_view() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        let result = query_crate_dep_counts(&db).expect("query should work");
        assert!(result.is_empty());
    }

    #[test]
    fn query_crate_dep_counts_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE VIEW crate_dependencies AS \
             SELECT * FROM (VALUES \
                 ('ops-core', 'serde', '^1.0', 'normal', false), \
                 ('ops-core', 'anyhow', '^1.0', 'normal', false), \
                 ('ops-core', 'tempfile', '^3.0', 'dev', false), \
                 ('ops-cli', 'clap', '^4.0', 'normal', false) \
             ) AS t(crate_name, dependency_name, version_req, dependency_kind, is_optional)",
        )
        .expect("create view with test data");
        drop(conn);

        let result = query_crate_dep_counts(&db).expect("query should work");
        assert_eq!(result.len(), 2);
        assert_eq!(result["ops-core"], 2); // only normal deps
        assert_eq!(result["ops-cli"], 1);
    }

    #[test]
    fn query_project_loc_no_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let loc = query_project_loc(&db).expect("query should work");
        assert_eq!(loc, 0);
    }

    #[test]
    fn query_project_file_count_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
             comments BIGINT, blanks BIGINT, lines BIGINT);
             INSERT INTO tokei_files VALUES ('Rust', 'src/main.rs', 500, 50, 20, 570);
             INSERT INTO tokei_files VALUES ('Rust', 'src/lib.rs', 300, 30, 10, 340);
             INSERT INTO tokei_files VALUES ('TOML', 'Cargo.toml', 40, 5, 3, 48);",
        )
        .expect("insert test data");
        drop(conn);

        let count = query_project_file_count(&db).expect("query should work");
        assert_eq!(count, 3);
    }

    #[test]
    fn query_project_file_count_no_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let count = query_project_file_count(&db).expect("query should work");
        assert_eq!(count, 0);
    }

    #[test]
    fn query_crate_file_count_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE tokei_files (language VARCHAR, file VARCHAR, code BIGINT, \
             comments BIGINT, blanks BIGINT, lines BIGINT);
             INSERT INTO tokei_files VALUES ('Rust', 'crates/my-lib/src/lib.rs', 3000, 200, 100, 3300);
             INSERT INTO tokei_files VALUES ('Rust', 'crates/my-lib/src/utils.rs', 1231, 50, 30, 1311);
             INSERT INTO tokei_files VALUES ('Rust', 'crates/my-cli/src/main.rs', 1892, 100, 50, 2042);",
        )
        .expect("insert test data");
        drop(conn);

        let result = query_crate_file_count(&db, &["crates/my-lib", "crates/my-cli"])
            .expect("query should work");
        assert_eq!(result.len(), 2);
        assert_eq!(result["crates/my-lib"], 2);
        assert_eq!(result["crates/my-cli"], 1);
    }

    #[test]
    fn query_crate_file_count_empty() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let result = query_crate_file_count(&db, &["crates/my-lib"]).expect("query should work");
        assert_eq!(result["crates/my-lib"], 0);
    }

    #[test]
    fn query_project_coverage_no_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let cov = query_project_coverage(&db).expect("query should work");
        assert_eq!(cov.lines_count, 0);
        assert_eq!(cov.lines_covered, 0);
        assert!((cov.lines_percent - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn query_project_coverage_with_data() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE coverage_files (filename VARCHAR, lines_count BIGINT, \
             lines_covered BIGINT, lines_percent DOUBLE);
             INSERT INTO coverage_files VALUES ('crates/core/src/lib.rs', 100, 80, 80.0);
             INSERT INTO coverage_files VALUES ('crates/cli/src/main.rs', 200, 150, 75.0);",
        )
        .expect("insert test data");
        drop(conn);

        let cov = query_project_coverage(&db).expect("query should work");
        assert_eq!(cov.lines_count, 300);
        assert_eq!(cov.lines_covered, 230);
        // 230/300 * 100 = 76.67
        assert!((cov.lines_percent - 76.67).abs() < 0.01);
    }

    #[test]
    fn query_crate_coverage_no_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let result =
            query_crate_coverage(&db, &["crates/core"], "/workspace").expect("query should work");
        assert_eq!(result["crates/core"].lines_count, 0);
    }

    #[test]
    fn query_crate_coverage_empty_members() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let result = query_crate_coverage(&db, &[], "/workspace").expect("query should work");
        assert!(result.is_empty());
    }

    #[test]
    fn query_crate_coverage_with_relative_paths() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE coverage_files (filename VARCHAR, lines_count BIGINT, \
             lines_covered BIGINT, lines_percent DOUBLE);
             INSERT INTO coverage_files VALUES ('crates/core/src/lib.rs', 100, 80, 80.0);
             INSERT INTO coverage_files VALUES ('crates/core/src/util.rs', 50, 40, 80.0);
             INSERT INTO coverage_files VALUES ('crates/cli/src/main.rs', 200, 150, 75.0);",
        )
        .expect("insert test data");
        drop(conn);

        let result = query_crate_coverage(&db, &["crates/core", "crates/cli"], "/workspace")
            .expect("query should work");

        assert_eq!(result.len(), 2);
        assert_eq!(result["crates/core"].lines_count, 150);
        assert_eq!(result["crates/core"].lines_covered, 120);
        assert_eq!(result["crates/cli"].lines_count, 200);
        assert_eq!(result["crates/cli"].lines_covered, 150);
    }

    #[test]
    fn query_crate_coverage_with_absolute_paths() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");

        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE coverage_files (filename VARCHAR, lines_count BIGINT, \
             lines_covered BIGINT, lines_percent DOUBLE);
             INSERT INTO coverage_files VALUES ('/workspace/crates/core/src/lib.rs', 100, 90, 90.0);
             INSERT INTO coverage_files VALUES ('/workspace/crates/cli/src/main.rs', 200, 100, 50.0);",
        )
        .expect("insert test data");
        drop(conn);

        let result = query_crate_coverage(&db, &["crates/core", "crates/cli"], "/workspace")
            .expect("query should work");

        assert_eq!(result.len(), 2);
        assert_eq!(result["crates/core"].lines_count, 100);
        assert_eq!(result["crates/core"].lines_covered, 90);
        assert_eq!(result["crates/cli"].lines_count, 200);
        assert_eq!(result["crates/cli"].lines_covered, 100);
    }
}
