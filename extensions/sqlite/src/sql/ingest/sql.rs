//! SQL builders and table-state probes for ingestor pipelines.
//!
//! # SEC-25 / TASK-2067 residual: closed by the SQLite port
//!
//! The `DuckDB` builder interpolated the staged JSON path into
//! `read_json_auto('<path>')` — the one staged read that could not go through
//! the verified [`crate::IngestDir`] anchor, narrowed only by an inode
//! re-check immediately before execution. SQLite's parameter binding removes
//! the residual entirely: the staged bytes are read through
//! [`crate::IngestDir::open_read`] in Rust and handed to the engine as a
//! bound `?1` parameter. No path reaches SQL, so there is no name to swap.

use crate::error::{DbError, DbResult};
use crate::sql::validation::{quoted_ident, TableName};
use crate::Sqlite;
use std::io::Read;

/// A table-shape DDL batch (`DROP TABLE IF EXISTS …; CREATE TABLE …`)
/// produced by a validated builder.
///
/// SEC-12 / TASK-1864: `load_with_sidecar` and the metadata ingestor execute
/// caller-supplied DDL, so the statement must be a gated newtype rather than
/// a bare `&str`. There is deliberately **no** public constructor taking a
/// `String` or `&str`: the only way to obtain this type is
/// [`JsonTableLoad::create_table_sql`], which builds the statement from
/// const-validated [`TableName`] / [`JsonColumn`] parts.
///
/// READ-8 / SQLite port note: the value is a *batch* (two statements)
/// because SQLite has no `CREATE OR REPLACE` — callers must execute it via
/// `Connection::execute_batch`, not `Connection::execute` (which rejects
/// multiple statements).
#[derive(Debug, Clone)]
#[must_use = "SEC-12: the built statement is the only gated form; discarding it means nothing is executed"]
pub struct CreateTableSql(String);

impl CreateTableSql {
    /// Borrow the built statement batch (for logging, assertions, and
    /// `execute_batch`).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for CreateTableSql {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A view DDL batch (`DROP VIEW IF EXISTS …; CREATE VIEW …`) produced by a
/// validated builder.
///
/// SEC-12 / TASK-1864: the companion of [`CreateTableSql`]. Being a distinct
/// type is load-bearing — the two statements are positional arguments of
/// `load_with_sidecar`, and swapping them must be a type error (API-2), not a
/// confusing `"{name} create"` error label at runtime.
///
/// SQLite has no `CREATE OR REPLACE VIEW`, so the builder emits DROP-then-
/// CREATE. `load_with_sidecar` already owns the whole load under the
/// connection lock and the database is a disposable cache, so the transient
/// no-view window between the two statements is not observable.
#[derive(Debug, Clone)]
#[must_use = "SEC-12: the built statement is the only gated form; discarding it means nothing is executed"]
pub struct CreateViewSql(String);

impl CreateViewSql {
    /// Build `DROP VIEW IF EXISTS <view>; CREATE VIEW <view> AS <body>`.
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
            "DROP VIEW IF EXISTS {view}; CREATE VIEW {view} AS {body}",
            view = view.quoted(),
            body = body.replace("<source>", &source.quoted())
        ))
    }

    /// Borrow the built statement batch (for logging, assertions, and
    /// `execute_batch`).
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

/// The SQL column type a JSON value is cast to on insert.
///
/// SQLite columns are dynamically typed; declaring the affinity in the DDL
/// *and* casting in the `INSERT … SELECT` keeps the table's contents as
/// typed as `DuckDB`'s inferred columns were, so downstream queries decode
/// `row.get::<_, i64>` / `f64` / `String` without per-row flexibility
/// surprises.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonColumnType {
    /// UTF-8 text.
    Text,
    /// 64-bit signed integer.
    Integer,
    /// 64-bit IEEE float.
    Real,
}

impl JsonColumnType {
    /// The DDL type name and `CAST` target — identical strings, one source
    /// of truth so the declared column and the inserted value can never
    /// disagree.
    #[must_use]
    pub const fn sql_name(self) -> &'static str {
        match self {
            Self::Text => "TEXT",
            Self::Integer => "INTEGER",
            Self::Real => "REAL",
        }
    }
}

/// One column of a flat-array JSON load: the table column name, the JSON key
/// it is populated from, and the type it is cast to.
///
/// SEC-12: construction is a `const fn` that asserts both the column name
/// (`[A-Za-z_][A-Za-z0-9_]*`, same rule as [`TableName`]) and the JSON path
/// (`$.<identifier>` — single-level, because every collector stages flat
/// one-record-per-row arrays; a nested path would signal a collector shape
/// change, not something to paper over here). Used in a `const`/`static`
/// context an invalid value is a build error, mirroring
/// [`TableName::from_static`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonColumn {
    name: &'static str,
    json_path: &'static str,
    col_type: JsonColumnType,
}

impl JsonColumn {
    /// Const-validating constructor; see the type docs. In a `const` context
    /// an invalid `name` or `json_path` fails the build.
    ///
    /// # Panics
    ///
    /// If `name` is not a valid SQL identifier or `json_path` is not a
    /// `$.<identifier>` single-level path. In a `const` context this is a
    /// compile-time error.
    #[must_use]
    pub const fn new(
        name: &'static str,
        json_path: &'static str,
        col_type: JsonColumnType,
    ) -> Self {
        assert!(
            is_valid_column_name_const(name),
            "JsonColumn name must be a valid SQL identifier ([A-Za-z_][A-Za-z0-9_]*)"
        );
        assert!(
            is_valid_json_path_const(json_path),
            "JsonColumn json_path must be a single-level '$.<identifier>' path"
        );
        Self {
            name,
            json_path,
            col_type,
        }
    }

    /// The validated column name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// The validated JSON path (`$.<key>`).
    #[must_use]
    pub const fn json_path(&self) -> &'static str {
        self.json_path
    }

    /// The column's SQL type.
    #[must_use]
    pub const fn col_type(&self) -> JsonColumnType {
        self.col_type
    }

    /// Convenience constructors for the three column types, so spec tables
    /// read as declarative data at the call site.
    #[must_use]
    pub const fn text(name: &'static str, json_path: &'static str) -> Self {
        Self::new(name, json_path, JsonColumnType::Text)
    }

    /// [`JsonColumn::text`] with an [`JsonColumnType::Integer`] type.
    #[must_use]
    pub const fn integer(name: &'static str, json_path: &'static str) -> Self {
        Self::new(name, json_path, JsonColumnType::Integer)
    }

    /// [`JsonColumn::text`] with an [`JsonColumnType::Real`] type.
    #[must_use]
    pub const fn real(name: &'static str, json_path: &'static str) -> Self {
        Self::new(name, json_path, JsonColumnType::Real)
    }
}

const fn is_valid_column_name_const(s: &str) -> bool {
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

const fn is_valid_json_path_const(s: &str) -> bool {
    // "$." + at least one identifier character, walked with slice patterns
    // (const-stable, and index/arithmetic-free — clippy-clean in const fn).
    match s.as_bytes() {
        [b'$', b'.', tail @ ..] => is_nonempty_identifier_bytes(tail),
        _ => false,
    }
}

/// `true` when `bytes` is non-empty and every byte is `[A-Za-z0-9_]` —
/// the tail rule shared by column names and single-level JSON paths.
const fn is_nonempty_identifier_bytes(mut bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    while let [b, tail @ ..] = bytes {
        if !(b.is_ascii_alphanumeric() || *b == b'_') {
            return false;
        }
        bytes = tail;
    }
    true
}

/// How a staged JSON file maps onto a table's rows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonLoadShape {
    /// The staged file is a JSON **array of flat objects** — one table row
    /// per element, one column per [`JsonColumn`] spec. This is the shape
    /// the `tokei`, `rust-loc` and `test-coverage` collectors stage: they
    /// flatten their reports to one record per row before writing the
    /// sidecar JSON.
    FlatArray(&'static [JsonColumn]),
    /// The staged file is a **single JSON object** stored verbatim in one
    /// `json TEXT NOT NULL` column (exactly one row). This is the shape of
    /// the `metadata` collector: the whole `cargo metadata` payload, nested
    /// structure intact, for JSON1 views to tear apart with `json_each`.
    SingleObjectBlob,
}

/// Declarative description of one table load from a staged JSON file.
///
/// Replaces the `DuckDB` `read_json_auto('<path>')` builder: instead of
/// handing the engine a path and letting it infer the shape, the collector
/// declares its record shape as a `const` spec and the engine receives the
/// file's *bytes* as a bound parameter. The SEC-12 contract shifts from
/// "validate the path before interpolating it" to "never interpolate a path
/// at all".
#[derive(Debug, Clone, Copy)]
#[must_use = "the load spec describes a table to create; discarding it loads nothing"]
pub struct JsonTableLoad {
    table: TableName,
    shape: JsonLoadShape,
}

impl JsonTableLoad {
    /// A load whose staged file is a flat array of objects (one row per
    /// element, one column per spec entry).
    pub const fn flat_array(table: &'static str, columns: &'static [JsonColumn]) -> Self {
        Self {
            table: TableName::from_static(table),
            shape: JsonLoadShape::FlatArray(columns),
        }
    }

    /// A load storing a single JSON object verbatim in a one-row
    /// `json TEXT NOT NULL` table.
    pub const fn single_object_blob(table: &'static str) -> Self {
        Self {
            table: TableName::from_static(table),
            shape: JsonLoadShape::SingleObjectBlob,
        }
    }

    /// The const-validated table name this load writes.
    #[must_use]
    pub const fn table(&self) -> TableName {
        self.table
    }

    /// The load's row shape.
    #[must_use]
    pub const fn shape(&self) -> &JsonLoadShape {
        &self.shape
    }

    /// Build the idempotent DDL batch for this load: drop the table if it
    /// exists, then create it with the spec's typed columns.
    ///
    /// SEC-12: the emitted statement is the gated [`CreateTableSql`]
    /// newtype; every identifier in it was validated at `const` construction
    /// time and is emitted double-quoted as defense in depth (the same
    /// posture `quoted_ident` establishes elsewhere).
    pub fn create_table_sql(&self) -> CreateTableSql {
        let quoted = self.table.quoted();
        let columns = match self.shape {
            JsonLoadShape::FlatArray(cols) => cols
                .iter()
                .map(|c| format!("\"{}\" {} NOT NULL", c.name, c.col_type.sql_name()))
                .collect::<Vec<_>>()
                .join(", "),
            JsonLoadShape::SingleObjectBlob => "\"json\" TEXT NOT NULL".to_string(),
        };
        CreateTableSql(format!(
            "DROP TABLE IF EXISTS {quoted}; CREATE TABLE {quoted} ({columns})"
        ))
    }

    /// Build the parameter-bound insert statement for this load.
    ///
    /// The staged JSON arrives as `?1` — a value, never interpolated SQL.
    /// For [`JsonLoadShape::FlatArray`] each element of `json_each(?1)`
    /// becomes one row (`CAST(json_extract(value, '<path>') AS <type>)` per
    /// column); for [`JsonLoadShape::SingleObjectBlob`] the whole payload is
    /// stored as the single `json` cell, with `json(?1)` validating it parses.
    ///
    /// Private by design: execution goes through [`execute_json_load`], so
    /// statement assembly and parameter binding cannot drift apart at a call
    /// site.
    fn insert_sql(&self) -> String {
        let quoted = self.table.quoted();
        match self.shape {
            JsonLoadShape::FlatArray(cols) => {
                let select_list = cols
                    .iter()
                    .map(|c| {
                        format!(
                            "CAST(json_extract(value, '{}') AS {})",
                            c.json_path,
                            c.col_type.sql_name()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                let column_list = cols
                    .iter()
                    .map(|c| format!("\"{}\"", c.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "INSERT INTO {quoted} ({column_list}) SELECT {select_list} FROM json_each(?1)"
                )
            }
            JsonLoadShape::SingleObjectBlob => {
                format!("INSERT INTO {quoted} (json) VALUES (json(?1))")
            }
        }
    }
}

/// Read the staged JSON through the anchor and load it into `conn`.
///
/// The full pipeline for one [`JsonTableLoad`]:
///
/// 1. read the staged file via [`crate::IngestDir::open_read`] (the verified
///    directory descriptor — the bytes loaded are the bytes this pipeline
///    staged),
/// 2. execute the DDL batch (`execute_batch`; two statements),
/// 3. execute the insert with the file's bytes bound as `?1`.
///
/// # Errors
///
/// [`DbError::Io`] if the staged file cannot be read through the anchor, or
/// [`DbError::QueryFailed`] if the DDL or the insert fails (malformed JSON,
/// a record missing a `NOT NULL` column's key, …).
pub fn execute_json_load(
    conn: &rusqlite::Connection,
    dir: &crate::sql::IngestDir,
    load: &JsonTableLoad,
    json_filename: &str,
) -> DbResult<()> {
    let mut json = String::new();
    // ERR-13: the anchored `open_read` reports the raw syscall error, which
    // names no file — wrap it so the operator sees which staged entry failed
    // (a missing `coverage_files.json` must say so).
    dir.open_read(json_filename)
        .map_err(|e| match e {
            DbError::Io(io) => DbError::Io(std::io::Error::new(
                io.kind(),
                format!("opening staged {json_filename} through the ingest anchor: {io}"),
            )),
            other => other,
        })?
        .read_to_string(&mut json)
        .map_err(|e| {
            DbError::Io(std::io::Error::new(
                e.kind(),
                format!("reading staged {json_filename}: {e}"),
            ))
        })?;
    load_json_string(conn, load, &json)
}

/// Load an already-read staged payload into `conn`.
///
/// The DDL-batch + bound-`?1`-insert half of [`execute_json_load`], exposed
/// for callers that must inspect the payload **before** it reaches the engine
/// — the metadata ingestor reads the staged bytes once so the
/// `OPS_METADATA_MAX_BYTES` cap is enforced in Rust (the SQLite port's
/// successor to the engine-side `maximum_object_size` option), then hands the
/// same string here rather than reading the file a second time.
///
/// SEC-12 note: `json` is a *parameter value*, never statement text — this
/// adds no SQL-assembly surface.
///
/// # Errors
///
/// [`DbError::QueryFailed`] if the DDL or the insert fails (malformed JSON, a
/// record missing a `NOT NULL` column's key, …).
pub fn load_json_string(
    conn: &rusqlite::Connection,
    load: &JsonTableLoad,
    json: &str,
) -> DbResult<()> {
    let label = load.table.as_str();
    conn.execute_batch(load.create_table_sql().as_str())
        .map_err(|e| DbError::query_failed(format!("{label} create"), e))?;
    conn.execute(&load.insert_sql(), rusqlite::params![json])
        .map_err(|e| DbError::query_failed(format!("{label} insert"), e))?;
    Ok(())
}

/// Check if a table or view exists in the database.
///
/// SQLite's `sqlite_master` catalog lists tables and views in one place, so
/// view-backed data sources (e.g. `crate_dependencies`) are detected along
/// with base tables (READ-5).
pub fn table_exists(conn: &rusqlite::Connection, table_name: &str) -> Result<bool, anyhow::Error> {
    use anyhow::Context;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE name = ?",
            rusqlite::params![table_name],
            |row: &rusqlite::Row<'_>| row.get(0),
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
pub fn table_has_data(db: &Sqlite, table_name: &str) -> Result<bool, anyhow::Error> {
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
            |row: &rusqlite::Row<'_>| row.get(0),
        )
        // ERR-7 (TASK-0521): Debug-format the table name to defang
        // control-character/log-injection.
        .with_context(|| format!("counting rows in {table_name:?}"))?;
    drop(conn);
    Ok(row_count > 0)
}

/// DUP-031: Generic helper to query rows from SQLite and return as a JSON array.
///
/// # Errors
///
/// If the database lock is poisoned, `sql` fails to prepare or execute, or
/// `row_mapper` fails on any row.
pub fn query_rows_to_json<F>(
    db: &Sqlite,
    sql: &str,
    row_mapper: F,
) -> Result<serde_json::Value, anyhow::Error>
where
    F: Fn(&rusqlite::Row<'_>) -> Result<serde_json::Value, rusqlite::Error>,
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

    #[test]
    fn table_has_data_no_table() {
        let db = Sqlite::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        let result = table_has_data(&db, "nonexistent_table").expect("should succeed");
        assert!(!result);
    }

    #[test]
    fn table_has_data_empty_table() {
        let db = Sqlite::open_in_memory().expect("open in-memory db");
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
        let db = Sqlite::open_in_memory().expect("open in-memory db");
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
        let db = Sqlite::open_in_memory().expect("open in-memory db");
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

    // --- JsonTableLoad builder tests (SEC-12 successors of the
    // create_table_from_json_sql tests) ---

    /// The tokei column spec, in the shape `extensions/tokei` will declare
    /// it in Phase 2 — kept here so the builder is tested against the real
    /// consumer spec, not a toy.
    const TOKEI_COLUMNS: &[JsonColumn] = &[
        JsonColumn::text("language", "$.language"),
        JsonColumn::text("file", "$.file"),
        JsonColumn::integer("code", "$.code"),
        JsonColumn::integer("comments", "$.comments"),
        JsonColumn::integer("blanks", "$.blanks"),
        JsonColumn::integer("lines", "$.lines"),
    ];

    /// SEC-12 (TASK-0522): the generated DDL wraps every identifier in
    /// double quotes — defense in depth that survives a future widening of
    /// the validators.
    #[test]
    fn create_table_sql_quotes_identifiers_and_declares_types() {
        let load = JsonTableLoad::flat_array("tokei_files", TOKEI_COLUMNS);
        let sql = load.create_table_sql();
        assert!(
            sql.as_str()
                .contains("DROP TABLE IF EXISTS \"tokei_files\";"),
            "expected drop+create batch, got: {sql}"
        );
        assert!(
            sql.as_str()
                .contains("CREATE TABLE \"tokei_files\" (\"language\" TEXT NOT NULL"),
            "expected quoted, typed columns in: {sql}"
        );
        assert!(
            sql.as_str().contains("\"code\" INTEGER NOT NULL"),
            "expected integer affinity in: {sql}"
        );
        assert!(
            !sql.as_str().contains("CREATE TABLE tokei_files "),
            "bare identifier interpolation regressed: {sql}"
        );
    }

    /// SEC-12: the const constructors reject malformed parts at build time.
    /// `TableName::from_static`'s own tests cover the table name; these pin
    /// the column gate via runtime calls to the same `const fn` body.
    #[test]
    fn json_column_rejects_malformed_parts() {
        // Valid shapes must not panic.
        let _ = JsonColumn::new("code", "$.code", JsonColumnType::Integer);
        // Invalid name: the assert fires (documented panic contract).
        let result = std::panic::catch_unwind(|| {
            JsonColumn::new("table; DROP", "$.code", JsonColumnType::Integer)
        });
        assert!(result.is_err(), "invalid column name must be rejected");
        // Invalid paths: not `$.<ident>` single-level.
        for bad in ["code", "$.", "$.a.b", "$.a[0]", "$.'x'", "$.a-b"] {
            let result =
                std::panic::catch_unwind(|| JsonColumn::new("c", bad, JsonColumnType::Text));
            assert!(result.is_err(), "json path {bad:?} must be rejected");
        }
    }

    /// Flat-array end-to-end on an in-memory database: DDL batch + bound
    /// `json_each` insert lands one typed row per staged record.
    #[test]
    fn flat_array_load_round_trips_typed_rows() {
        let dir_tmp = tempfile::tempdir().expect("tempdir");
        let dir = crate::sql::IngestDir::open(&dir_tmp.path().join("data.db.ingest"))
            .expect("open ingest dir");
        dir.write_atomic(
            "tokei_files.json",
            br#"[{"language":"Rust","file":"src/lib.rs","code":10,"comments":2,"blanks":1,"lines":13},
                 {"language":"TOML","file":"Cargo.toml","code":5,"comments":0,"blanks":0,"lines":5}]"#,
        )
        .expect("stage json");

        let db = Sqlite::open_in_memory().expect("db");
        let conn = db.lock().expect("lock");
        let load = JsonTableLoad::flat_array("tokei_files", TOKEI_COLUMNS);
        execute_json_load(&conn, &dir, &load, "tokei_files.json").expect("load");

        let (language, code): (String, i64) = conn
            .query_row(
                "SELECT language, code FROM tokei_files WHERE file = 'src/lib.rs'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("select typed row");
        assert_eq!(language, "Rust");
        assert_eq!(code, 10);
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM tokei_files", [], |row| row.get(0))
            .expect("count");
        assert_eq!(count, 2);
        drop(conn);
    }

    /// A record missing a `NOT NULL` column's key fails the insert loudly —
    /// drift between collector output and the declared spec surfaces as an
    /// ingest error, never as a silent NULL column.
    #[test]
    fn flat_array_load_rejects_record_missing_a_column() {
        let dir_tmp = tempfile::tempdir().expect("tempdir");
        let dir = crate::sql::IngestDir::open(&dir_tmp.path().join("data.db.ingest"))
            .expect("open ingest dir");
        dir.write_atomic(
            "tokei_files.json",
            br#"[{"language":"Rust","file":"src/lib.rs"}]"#,
        )
        .expect("stage json");

        let db = Sqlite::open_in_memory().expect("db");
        let conn = db.lock().expect("lock");
        let load = JsonTableLoad::flat_array("tokei_files", TOKEI_COLUMNS);
        let err = execute_json_load(&conn, &dir, &load, "tokei_files.json")
            .expect_err("missing NOT NULL key must fail");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("tokei_files insert"),
            "error must name the failing step: {rendered}"
        );
        drop(conn);
    }

    /// The single-object-blob shape stores the whole payload verbatim in one
    /// row; `json(?1)` rejects a payload that does not parse.
    #[test]
    fn single_object_blob_round_trips_and_rejects_non_json() {
        let dir_tmp = tempfile::tempdir().expect("tempdir");
        let dir = crate::sql::IngestDir::open(&dir_tmp.path().join("data.db.ingest"))
            .expect("open ingest dir");
        dir.write_atomic(
            "metadata.json",
            br#"{"workspace_root":"/ws","packages":[{"name":"a"}]}"#,
        )
        .expect("stage metadata");

        let db = Sqlite::open_in_memory().expect("db");
        let conn = db.lock().expect("lock");
        let load = JsonTableLoad::single_object_blob("metadata_raw");
        execute_json_load(&conn, &dir, &load, "metadata.json").expect("load");

        let packages: i64 = conn
            .query_row(
                "SELECT json_array_length(json, '$.packages') FROM metadata_raw",
                [],
                |row| row.get(0),
            )
            .expect("json1 over blob");
        assert_eq!(packages, 1);

        // Non-JSON payload: json(?1) must refuse it.
        dir.write_atomic("bad.json", b"not json at all")
            .expect("stage bad");
        let err = execute_json_load(&conn, &dir, &load, "bad.json")
            .expect_err("non-JSON payload must fail");
        assert!(
            format!("{err:#}").contains("metadata_raw insert"),
            "error names the insert: {err:#}"
        );
        drop(conn);
    }

    /// SEC-12 / TASK-1864: pins that an unvalidated string cannot reach
    /// `conn.execute` through this crate's public API.
    ///
    /// `SidecarIngestorConfig::load_with_sidecar` is the public entry point
    /// that executes caller-supplied DDL, and it takes a `&JsonTableLoad`
    /// plus `&CreateViewSql`. Neither has a public constructor accepting a
    /// `String` or `&str`:
    ///
    /// - `JsonTableLoad` comes only from `flat_array` / `single_object_blob`,
    ///   whose table name is const-validated and whose columns are
    ///   const-validated `JsonColumn`s.
    /// - `CreateViewSql` comes only from `create_or_replace`, whose two
    ///   identifiers are const-validated `TableName`s and whose body is
    ///   `&'static str`.
    /// - The `from_literal_for_tests` escape hatches are `#[cfg(test)]`, so
    ///   they do not exist in a compiled library.
    ///
    /// The DDL and insert types are also distinct, so the positional
    /// arguments of `load_with_sidecar` can no longer be swapped at a call
    /// site (API-2).
    #[test]
    fn unvalidated_sql_cannot_reach_conn_execute_through_the_public_api() {
        // Every const constructor below validates at compile time; calling
        // them at runtime exercises the same assert. Malformed parts panic
        // rather than producing a value of the type.
        // Named const rather than an inline `&[…]`: `flat_array` takes
        // `&'static [JsonColumn]` and const-fn calls are not rvalue-promotable.
        const COLS: &[JsonColumn] = &[JsonColumn::integer("i", "$.i")];
        let load = JsonTableLoad::flat_array("t", COLS);
        assert!(load.create_table_sql().as_str().contains("\"t\""));
        let _ = JsonTableLoad::single_object_blob("metadata_raw");
    }

    // --- query_rows_to_json (TEST-5 / TASK-1870) ---
    //
    // Three downstream crates (`extensions/tokei`, `extensions-rust/loc`,
    // `extensions-rust/test-coverage`) turn query results into JSON through
    // this helper, and every one of them degrades softly on a break: an
    // empty array renders as "no data" on the about page rather than as a
    // failed run. These tests pin the concrete shapes.

    fn seed_rows_table(db: &Sqlite, values: &str) {
        let conn = db.lock().expect("lock");
        conn.execute_batch(&format!(
            "CREATE TABLE rows_src (id INTEGER, label TEXT); {values}"
        ))
        .expect("seed");
        drop(conn);
    }

    #[test]
    fn query_rows_to_json_maps_every_row_of_a_populated_table() {
        let db = Sqlite::open_in_memory().expect("open in-memory db");
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
        let db = Sqlite::open_in_memory().expect("open in-memory db");
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
        let db = Sqlite::open_in_memory().expect("open in-memory db");
        seed_rows_table(&db, "INSERT INTO rows_src VALUES (1, 'one');");

        // Decoding a TEXT column as i32 is the row mapper's error path.
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
    /// only free-form part of the statement is a `&'static str`. SQLite has
    /// no `CREATE OR REPLACE VIEW`, so the batch drops first.
    #[test]
    fn create_view_sql_quotes_identifiers_and_substitutes_source() {
        let sql = CreateViewSql::create_or_replace(
            TableName::from_static("tokei_languages"),
            TableName::from_static("tokei_files"),
            "SELECT language FROM <source> GROUP BY language",
        );
        assert_eq!(
            sql.as_str(),
            "DROP VIEW IF EXISTS \"tokei_languages\"; \
             CREATE VIEW \"tokei_languages\" AS \
             SELECT language FROM \"tokei_files\" GROUP BY language"
        );
    }

    /// The DROP+CREATE view batch is idempotent on a live database — the
    /// transient no-view window is not observable to a fresh query.
    #[test]
    fn create_view_sql_batch_replaces_an_existing_view() {
        let db = Sqlite::open_in_memory().expect("db");
        let conn = db.lock().expect("lock");
        conn.execute_batch("CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (1)")
            .expect("seed");
        let sql = CreateViewSql::create_or_replace(
            TableName::from_static("v"),
            TableName::from_static("t"),
            "SELECT COUNT(*) AS n FROM <source>",
        );
        conn.execute_batch(sql.as_str()).expect("first create");
        conn.execute_batch(sql.as_str())
            .expect("replace must succeed");
        let n: i64 = conn
            .query_row("SELECT n FROM v", [], |row| row.get(0))
            .expect("query view");
        assert_eq!(n, 1);
        drop(conn);
    }
}
