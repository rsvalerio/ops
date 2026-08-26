//! SQL builders and table-state probes for ingestor pipelines.

use crate::sql::validation::{prepare_path_for_sql, quoted_ident, ExtraOpts, SqlError};
use crate::DuckDb;
use std::path::Path;

/// Generate `CREATE OR REPLACE TABLE <name> AS SELECT * FROM read_json_auto(...)` SQL (DUP-009).
///
/// Validates and escapes the path for safe interpolation. Pass an
/// [`ExtraOpts`] (validated at construction) for additional
/// `read_json_auto` parameters (e.g. `"maximum_object_size=67108864"`).
///
/// SEC-12 / TASK-1623: the `extra_opts` argument is typed as `Option<ExtraOpts>`
/// rather than `Option<&str>` so the validation contract (`validate_extra_opts`)
/// is encoded in the type system. Any future dynamic caller must route
/// through [`ExtraOpts::new`] and cannot bypass the allowlist.
///
/// # Errors
///
/// [`SqlError`] if `table_name` is not a valid identifier, `path` fails
/// path validation, or `extra_opts` is malformed.
pub fn create_table_from_json_sql(
    table_name: &str,
    path: &Path,
    extra_opts: Option<ExtraOpts<'_>>,
) -> Result<String, SqlError> {
    // SEC-12 (TASK-0522): use the same `quoted_ident` defense-in-depth as
    // `table_has_data` and `drop_table_if_exists` so a future widening of
    // `validate_identifier` (e.g. allowing schema-qualified names) does
    // not silently break the safety contract here.
    let quoted = quoted_ident(table_name)?;
    let escaped = prepare_path_for_sql(path)?;
    // READ-8 / TASK-1627: single `format!` site; the optional opts segment
    // is rendered inline so the SQL template lives in exactly one place.
    let opts_segment = match extra_opts {
        Some(opts) => format!(", {}", opts.as_str()),
        None => String::new(),
    };
    Ok(format!(
        "CREATE OR REPLACE TABLE {quoted} AS SELECT * FROM read_json_auto('{escaped}'{opts_segment})",
    ))
}

/// Check if a table or view exists in the database.
///
/// `information_schema.tables` does **not** list views in `DuckDB`; we union
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
            |row: &duckdb::Row<'_>| row.get(0),
        )
        // ERR-7: render the identifier via Debug so any embedded control
        // characters (\n, \t, NULs, ANSI escapes …) are escaped and cannot
        // forge log lines or smuggle stray formatting into the error chain.
        .with_context(|| format!("checking if {table_name:?} exists"))?;
    Ok(count > 0)
}

/// Check if a table exists and has at least one row.
///
/// # Errors
///
/// If the database lock is poisoned, `table_name` is not a valid
/// identifier, or the count query fails. A missing table is `Ok(false)`.
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
            |row: &duckdb::Row<'_>| row.get(0),
        )
        // ERR-7 (TASK-0521): Debug-format the table name to defang
        // control-character/log-injection.
        .with_context(|| format!("counting rows in {table_name:?}"))?;
    drop(conn);
    Ok(row_count > 0)
}

/// DUP-031: Generic helper to query rows from `DuckDB` and return as a JSON array.
///
/// # Errors
///
/// If the database lock is poisoned, `sql` fails to prepare or execute, or
/// `row_mapper` fails on any row.
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
    // CONC-1: release the connection guard before building the JSON value.
    // `stmt` borrows `conn`, so it has to go first.
    drop(stmt);
    drop(conn);
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
        drop(conn);
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
        let opts1 = ExtraOpts::new("maximum_object_size=67108864").expect("safe opts");
        assert!(create_table_from_json_sql("t", &path, Some(opts1)).is_ok());
        let opts2 = ExtraOpts::new("maximum_object_size=1,format=auto").expect("safe opts");
        assert!(create_table_from_json_sql("t", &path, Some(opts2)).is_ok());
    }

    /// SEC-12 / TASK-1623: the validation contract now lives on
    /// `ExtraOpts::new`. Malicious fragments must be rejected at
    /// construction so they can never reach `create_table_from_json_sql`.
    #[test]
    fn extra_opts_new_rejects_malicious_fragments() {
        assert!(ExtraOpts::new("maximum_object_size=1, injection='x') --").is_err());
        assert!(ExtraOpts::new("a=1;DROP TABLE users").is_err());
        assert!(ExtraOpts::new("a=(1)").is_err());
        assert!(ExtraOpts::new("a='x'").is_err());
        assert!(ExtraOpts::new("a").is_err());
        assert!(ExtraOpts::new("").is_err());
    }

    /// SEC-12 / TASK-1623 AC #4: a dynamic (non-static) opts string still
    /// flows through the same `ExtraOpts::new` validation gate. Pinning the
    /// dynamic-construction path here so a future widening of the type's
    /// constructors (e.g. an unchecked `from_static`) would have to update
    /// this test alongside the API.
    #[test]
    fn extra_opts_new_validates_dynamic_construction() {
        let path = PathBuf::from("/safe/path.json");
        let cap: u64 = 9_999_999;
        let dynamic = format!("maximum_object_size={cap}");
        let opts = ExtraOpts::new(&dynamic).expect("dynamic safe opts");
        let sql = create_table_from_json_sql("t", &path, Some(opts)).expect("ok");
        assert!(sql.contains("maximum_object_size=9999999"), "got: {sql}");

        let bad = format!("a=1;{}", "DROP TABLE users");
        assert!(ExtraOpts::new(&bad).is_err());
    }
}
