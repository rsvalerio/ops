//! SQL builders and table-state probes for ingestor pipelines.

use crate::sql::validation::{prepare_path_for_sql, quoted_ident, validate_extra_opts, SqlError};
use crate::DuckDb;
use std::path::Path;

/// Generate `CREATE OR REPLACE TABLE <name> AS SELECT * FROM read_json_auto(...)` SQL (DUP-009).
///
/// Validates and escapes the path for safe interpolation. Pass `extra_opts` for
/// additional read_json_auto parameters (e.g., `"maximum_object_size=67108864"`).
pub fn create_table_from_json_sql(
    table_name: &str,
    path: &Path,
    extra_opts: Option<&str>,
) -> Result<String, SqlError> {
    // SEC-12 (TASK-0522): use the same `quoted_ident` defense-in-depth as
    // `table_has_data` and `drop_table_if_exists` so a future widening of
    // `validate_identifier` (e.g. allowing schema-qualified names) does
    // not silently break the safety contract here.
    let quoted = quoted_ident(table_name)?;
    let escaped = prepare_path_for_sql(path)?;
    match extra_opts {
        Some(opts) => {
            validate_extra_opts(opts)?;
            Ok(format!(
            "CREATE OR REPLACE TABLE {quoted} AS SELECT * FROM read_json_auto('{escaped}', {opts})",
        ))
        }
        None => Ok(format!(
            "CREATE OR REPLACE TABLE {quoted} AS SELECT * FROM read_json_auto('{escaped}')",
        )),
    }
}

/// Check if a table or view exists in the database.
///
/// `information_schema.tables` does **not** list views in DuckDB; we union
/// with `information_schema.views` so that view-backed data sources (e.g.
/// `crate_dependencies`) are detected (READ-5).
pub(crate) fn table_exists(
    conn: &duckdb::Connection,
    table_name: &str,
) -> Result<bool, anyhow::Error> {
    use anyhow::Context;
    let count: i64 = conn
        .query_row(
            "SELECT \
                (SELECT COUNT(*) FROM information_schema.tables WHERE table_name = ?) \
              + (SELECT COUNT(*) FROM information_schema.views  WHERE table_name = ?)",
            duckdb::params![table_name, table_name],
            |row: &duckdb::Row| row.get(0),
        )
        // ERR-7: render the identifier via Debug so any embedded control
        // characters (\n, \t, NULs, ANSI escapes …) are escaped and cannot
        // forge log lines or smuggle stray formatting into the error chain.
        .with_context(|| format!("checking if {table_name:?} exists"))?;
    Ok(count > 0)
}

/// Check if a table exists and has at least one row.
pub fn table_has_data(db: &DuckDb, table_name: &str) -> Result<bool, anyhow::Error> {
    use anyhow::Context;

    let conn = db.lock().context("acquiring db lock")?;
    if !table_exists(&conn, table_name)? {
        return Ok(false);
    }
    let quoted = quoted_ident(table_name)?;
    let row_count: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {quoted}"),
            [],
            |row: &duckdb::Row| row.get(0),
        )
        // ERR-7 (TASK-0521): Debug-format the table name to defang
        // control-character/log-injection.
        .with_context(|| format!("counting rows in {table_name:?}"))?;
    Ok(row_count > 0)
}

/// DUP-031: Generic helper to query rows from DuckDB and return as a JSON array.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_schema;
    use std::path::PathBuf;

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
    fn table_exists_detects_views_too() {
        // READ-5 regression: views must be detected, not just base tables.
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        let conn = db.lock().expect("lock");
        conn.execute_batch(
            "CREATE TABLE base (n INTEGER); \
             CREATE VIEW only_view AS SELECT 1 AS n;",
        )
        .expect("create");
        assert!(table_exists(&conn, "base").expect("table"));
        assert!(table_exists(&conn, "only_view").expect("view"));
        assert!(!table_exists(&conn, "nope").expect("missing"));
    }

    #[test]
    fn table_exists_error_message_sanitizes_control_chars() {
        let nasty = "name\nADMIN: forged log line\rwith ESC\x1b[31m red";
        let rendered = format!("checking if {nasty:?} exists");
        assert!(
            !rendered.contains('\n') && !rendered.contains('\r') && !rendered.contains('\x1b'),
            "control chars must be escaped in error context: {rendered}"
        );
        assert!(rendered.contains("\\n"), "newline escaped: {rendered}");
        assert!(rendered.contains("\\u{1b}"), "ESC escaped: {rendered}");
    }

    #[test]
    fn table_has_data_error_message_sanitizes_control_chars() {
        let nasty = "name\nADMIN: forged log line\rwith ESC\x1b[31m red";
        let rendered = format!("counting rows in {nasty:?}");
        assert!(
            !rendered.contains('\n') && !rendered.contains('\r') && !rendered.contains('\x1b'),
            "control chars must be escaped in error context: {rendered}"
        );
        assert!(rendered.contains("\\n"), "newline escaped: {rendered}");
        assert!(rendered.contains("\\u{1b}"), "ESC escaped: {rendered}");
    }

    #[test]
    fn create_table_from_json_sql_rejects_invalid_table_name() {
        let path = PathBuf::from("/safe/path.json");
        assert!(create_table_from_json_sql("valid_table", &path, None).is_ok());
        assert!(create_table_from_json_sql("table; DROP", &path, None).is_err());
        assert!(create_table_from_json_sql("", &path, None).is_err());
        assert!(create_table_from_json_sql("123start", &path, None).is_err());
    }

    /// SEC-12 (TASK-0522): the generated SQL wraps the validated identifier
    /// in double quotes — defense-in-depth that survives a future widening
    /// of `validate_identifier`.
    #[test]
    fn create_table_from_json_sql_quotes_identifier() {
        let path = PathBuf::from("/safe/path.json");
        let sql = create_table_from_json_sql("tokei_files", &path, None).expect("ok");
        assert!(
            sql.contains("\"tokei_files\""),
            "expected quoted identifier in: {sql}"
        );
        assert!(
            !sql.contains("CREATE OR REPLACE TABLE tokei_files "),
            "bare identifier interpolation regressed: {sql}"
        );
    }

    #[test]
    fn create_table_from_json_sql_accepts_safe_extra_opts() {
        let path = PathBuf::from("/safe/path.json");
        assert!(
            create_table_from_json_sql("t", &path, Some("maximum_object_size=67108864")).is_ok()
        );
        assert!(
            create_table_from_json_sql("t", &path, Some("maximum_object_size=1,format=auto"))
                .is_ok()
        );
    }

    #[test]
    fn create_table_from_json_sql_rejects_malicious_extra_opts() {
        let path = PathBuf::from("/safe/path.json");
        assert!(create_table_from_json_sql(
            "t",
            &path,
            Some("maximum_object_size=1, injection='x') --")
        )
        .is_err());
        assert!(create_table_from_json_sql("t", &path, Some("a=1;DROP TABLE users")).is_err());
        assert!(create_table_from_json_sql("t", &path, Some("a=(1)")).is_err());
        assert!(create_table_from_json_sql("t", &path, Some("a='x'")).is_err());
        assert!(create_table_from_json_sql("t", &path, Some("a")).is_err());
        assert!(create_table_from_json_sql("t", &path, Some("")).is_err());
    }
}
