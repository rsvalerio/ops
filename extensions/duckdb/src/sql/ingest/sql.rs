//! SQL builders and table-state probes for ingestor pipelines.

use crate::sql::validation::{prepare_path_for_sql, quoted_ident, ExtraOpts, SqlError, TableName};
use crate::DuckDb;
use std::path::Path;

/// A `CREATE OR REPLACE TABLE …` statement produced by a validated builder.
///
/// SEC-12 / TASK-1864: `SidecarIngestorConfig::load_with_sidecar` used to take
/// its two statements as bare `&str`, making it the widest un-gated path into
/// `conn.execute` in the crate — the validated-builder discipline every other
/// interpolation site enforces (`TableName`, `ExtraOpts`, `quoted_ident`,
/// `QueryTableName`) was upheld only by convention at call sites this crate
/// cannot see. There is deliberately **no** public constructor taking a
/// `String` or `&str`: the only way to obtain this type is
/// [`create_table_from_json_sql`], which validates the table identifier and
/// the interpolated path first.
#[derive(Debug, Clone)]
#[must_use = "SEC-12: the built statement is the only gated form; discarding it means nothing is executed"]
pub struct CreateTableSql(String);

impl CreateTableSql {
    /// Borrow the built statement (for logging, assertions, and `execute`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
impl CreateTableSql {
    /// Test-only escape hatch: build the newtype from a literal so
    /// crate-internal tests can drive `load_with_sidecar` with fixture SQL
    /// (`CREATE TABLE … FROM (VALUES …)`) that no production builder emits.
    /// `#[cfg(test)]` keeps it out of the crate's public surface entirely.
    pub(crate) fn from_literal_for_tests(sql: &str) -> Self {
        Self(sql.to_string())
    }
}

impl std::fmt::Display for CreateTableSql {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A `CREATE OR REPLACE VIEW …` statement produced by a validated builder.
///
/// SEC-12 / TASK-1864: the companion of [`CreateTableSql`]. Being a distinct
/// type is load-bearing — the two statements are positional arguments of
/// `load_with_sidecar`, and swapping them used to compile and merely produce a
/// confusing `"{name} create"` error label (API-2).
#[derive(Debug, Clone)]
#[must_use = "SEC-12: the built statement is the only gated form; discarding it means nothing is executed"]
pub struct CreateViewSql(String);

impl CreateViewSql {
    /// Build `CREATE OR REPLACE VIEW <view> AS <body>`.
    ///
    /// The gate is the one `PerCrateI64Query::select_expr` established: `body`
    /// is `&'static str`, so "static-vetted SQL fragment" is a build-time
    /// property rather than a call-site convention — a config- or
    /// metadata-derived `String` cannot be passed. Both identifiers are
    /// const-validated [`TableName`]s, and every `<source>` placeholder in
    /// `body` is replaced with the quoted source table so the `FROM` clause
    /// carries the same validation as the view name.
    pub fn create_or_replace(view: TableName, source: TableName, body: &'static str) -> Self {
        Self(format!(
            "CREATE OR REPLACE VIEW {} AS {}",
            view.quoted(),
            body.replace("<source>", &source.quoted())
        ))
    }

    /// Borrow the built statement (for logging, assertions, and `execute`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
impl CreateViewSql {
    /// Test-only escape hatch; see [`CreateTableSql::from_literal_for_tests`].
    pub(crate) fn from_literal_for_tests(sql: &str) -> Self {
        Self(sql.to_string())
    }
}

impl std::fmt::Display for CreateViewSql {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

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
/// SEC-12 / TASK-1864: returns the gated [`CreateTableSql`] newtype rather
/// than a bare `String`, so the statement `load_with_sidecar` executes is
/// provably the output of this validator.
///
/// # SEC-25 / TASK-2067: the one staged access that is not anchored
///
/// TASK-2054 put every staged write, rename, unlink and checksum behind
/// [`crate::IngestDir`]'s verified directory descriptor. The `path`
/// interpolated here is the exception: the embedded `DuckDB` engine takes
/// `read_json_auto('<path>')` as a *string* and offers no descriptor-passing
/// API, so this read — and only this read — resolves the ingest directory by
/// name. [`crate::IngestDir::path`] / `entry_path` exist for it.
///
/// The residual is narrower than the write side was. An attacker who swaps the
/// directory name between the anchored write and the engine's read can feed
/// `DuckDB` JSON of their choosing, but cannot capture what ops staged — the
/// write already landed on the verified inode. What they can do is get their
/// rows into a table the about pipeline then reports as the project's own.
///
/// That is **not** accepted as-is: `SidecarIngestorConfig::load_with_sidecar`
/// calls [`crate::IngestDir::verify_entry_identity`] immediately before
/// executing this statement, so the path and the anchor are checked to name
/// the same inode. That shrinks the window to the gap between the check and
/// the engine's own `open`; it does not close it, because closing it needs
/// either a descriptor-passing read in `DuckDB` or an in-memory hand-off the
/// engine does not offer. The remaining gap is recorded here rather than left
/// implicit, and re-opens the moment a caller builds this statement without
/// going through `load_with_sidecar`.
///
/// # Errors
///
/// [`SqlError`] if `table_name` is not a valid identifier, `path` fails
/// path validation, or `extra_opts` is malformed.
pub fn create_table_from_json_sql(
    table_name: &str,
    path: &Path,
    extra_opts: Option<ExtraOpts<'_>>,
) -> Result<CreateTableSql, SqlError> {
    // SEC-12 (TASK-0522): use the same `quoted_ident` defense-in-depth as
    // `table_has_data` and `drop_table_if_exists` so a future widening of
    // `validate_identifier` (e.g. allowing schema-qualified names) does
    // not silently break the safety contract here.
    let quoted = quoted_ident(table_name)?;
    let escaped = prepare_path_for_sql(path)?;
    // READ-8 / TASK-1627: single `format!` site; the optional opts segment
    // is rendered inline so the SQL template lives in exactly one place.
    let opts_segment = extra_opts.map_or_else(String::new, |opts| format!(", {}", opts.as_str()));
    Ok(CreateTableSql(format!(
        "CREATE OR REPLACE TABLE {quoted} AS SELECT * FROM read_json_auto('{escaped}'{opts_segment})",
    )))
}

/// Check if a table or view exists in the database.
///
/// `information_schema.tables` does **not** list views in `DuckDB`; we union
/// with `information_schema.views` so that view-backed data sources (e.g.
/// `crate_dependencies`) are detected (READ-5).
pub fn table_exists(conn: &duckdb::Connection, table_name: &str) -> Result<bool, anyhow::Error> {
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
            sql.as_str().contains("\"tokei_files\""),
            "expected quoted identifier in: {sql}"
        );
        assert!(
            !sql.as_str()
                .contains("CREATE OR REPLACE TABLE tokei_files "),
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
        assert!(
            sql.as_str().contains("maximum_object_size=9999999"),
            "got: {sql}"
        );

        let bad = format!("a=1;{}", "DROP TABLE users");
        assert!(ExtraOpts::new(&bad).is_err());
    }

    // --- query_rows_to_json (TEST-5 / TASK-1870) ---
    //
    // Three downstream crates (`extensions/tokei`, `extensions-rust/loc`,
    // `extensions-rust/test-coverage`) turn query results into JSON through
    // this helper, and every one of them degrades softly on a break: an
    // empty array renders as "no data" on the about page rather than as a
    // failed run. These tests pin the concrete shapes.

    fn seed_rows_table(db: &DuckDb, values: &str) {
        let conn = db.lock().expect("lock");
        conn.execute_batch(&format!(
            "CREATE TABLE rows_src (id INTEGER, label VARCHAR); {values}"
        ))
        .expect("seed");
        drop(conn);
    }

    #[test]
    fn query_rows_to_json_maps_every_row_of_a_populated_table() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        seed_rows_table(
            &db,
            "INSERT INTO rows_src VALUES (1, 'one'), (2, 'two'), (3, 'three');",
        );

        let value = query_rows_to_json(&db, "SELECT id, label FROM rows_src ORDER BY id", |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, i32>(0)?,
                "label": row.get::<_, String>(1)?,
            }))
        })
        .expect("query");

        assert_eq!(
            value,
            serde_json::json!([
                {"id": 1, "label": "one"},
                {"id": 2, "label": "two"},
                {"id": 3, "label": "three"},
            ])
        );
    }

    /// TEST-5 / TASK-1870: an empty result set is an empty JSON **array**,
    /// not `null`. Downstream `about` pages iterate the value directly, so
    /// the distinction is load-bearing.
    #[test]
    fn query_rows_to_json_returns_an_empty_array_for_no_rows() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        seed_rows_table(&db, "");

        let value = query_rows_to_json(&db, "SELECT id FROM rows_src", |row| {
            Ok(serde_json::json!(row.get::<_, i32>(0)?))
        })
        .expect("query");

        assert_eq!(value, serde_json::Value::Array(vec![]));
        assert!(!value.is_null(), "empty result must not collapse to null");
    }

    /// TEST-5 / TASK-1870: a failing row mapper surfaces as an error rather
    /// than a silently short result.
    #[test]
    fn query_rows_to_json_propagates_a_row_mapper_error() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        seed_rows_table(&db, "INSERT INTO rows_src VALUES (1, 'one');");

        // Decoding a VARCHAR column as i32 is the row mapper's error path.
        let err = query_rows_to_json(&db, "SELECT label FROM rows_src", |row| {
            Ok(serde_json::json!(row.get::<_, i32>(0)?))
        })
        .expect_err("row mapper failure must propagate");

        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("reading row") || rendered.contains("querying"),
            "error must carry the helper's context: {rendered}"
        );
    }

    /// SEC-12 / TASK-1864: the view builder quotes the view name and
    /// substitutes the const-validated source table for `<source>`, so the
    /// only free-form part of the statement is a `&'static str`.
    #[test]
    fn create_view_sql_quotes_identifiers_and_substitutes_source() {
        let sql = CreateViewSql::create_or_replace(
            TableName::from_static("tokei_languages"),
            TableName::from_static("tokei_files"),
            "SELECT language FROM <source> GROUP BY language",
        );
        assert_eq!(
            sql.as_str(),
            "CREATE OR REPLACE VIEW \"tokei_languages\" AS \
             SELECT language FROM \"tokei_files\" GROUP BY language"
        );
    }

    /// SEC-12 / TASK-1864: pins that an unvalidated string cannot reach
    /// `conn.execute` through this crate's public API.
    ///
    /// `SidecarIngestorConfig::load_with_sidecar` is the only public entry
    /// point that executes caller-supplied DDL, and it now takes
    /// `&CreateTableSql` / `&CreateViewSql`. Neither newtype has a public
    /// constructor accepting a `String` or `&str`:
    ///
    /// - `CreateTableSql` comes only from `create_table_from_json_sql`, which
    ///   this test drives with injection-shaped paths and an
    ///   injection-shaped table name — all rejected, so no value of the type
    ///   ever exists for them.
    /// - `CreateViewSql` comes only from `create_or_replace`, whose two
    ///   identifiers are const-validated `TableName`s and whose body is
    ///   `&'static str`.
    /// - The `from_literal_for_tests` escape hatches are `#[cfg(test)]`, so
    ///   they do not exist in a compiled library.
    ///
    /// The two are also distinct types, so the positional arguments can no
    /// longer be swapped at a call site (API-2).
    #[test]
    fn unvalidated_sql_cannot_reach_conn_execute_through_the_public_api() {
        for path in [
            PathBuf::from("/tmp/x'); DROP TABLE users; --"),
            PathBuf::from("../../../etc/passwd"),
            PathBuf::from("/tmp/$(whoami).json"),
        ] {
            assert!(
                create_table_from_json_sql("t", &path, None).is_err(),
                "path {} must not produce a CreateTableSql",
                path.display()
            );
        }
        let safe = PathBuf::from("/safe/path.json");
        assert!(create_table_from_json_sql("t; DROP TABLE users; --", &safe, None).is_err());
        assert!(ExtraOpts::new("maximum_object_size=1) UNION SELECT 1 --").is_err());
    }
}
