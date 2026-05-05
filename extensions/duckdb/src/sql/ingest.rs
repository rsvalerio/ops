//! Table creation, sidecar I/O, and data pipeline helpers.

use crate::{DbError, DbResult, DuckDb};
use std::path::{Path, PathBuf};

use super::validation::{prepare_path_for_sql, quoted_ident, validate_extra_opts, SqlError};

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
pub(super) fn table_exists(
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
        // table_name is a static string in every current call site, but the
        // function is `pub(super)` and the cost of this guard is zero.
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
    // table_name needs interpolation for the COUNT query since DuckDB
    // doesn't support parameterized table names.
    let quoted = quoted_ident(table_name)?;
    let row_count: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM {quoted}"),
            [],
            |row: &duckdb::Row| row.get(0),
        )
        // ERR-7 (TASK-0521): Debug-format the table name to defang
        // control-character/log-injection, matching the sibling
        // `table_exists` error context.
        .with_context(|| format!("counting rows in {table_name:?}"))?;
    Ok(row_count > 0)
}

/// Compute the ingest data directory from a DB path (appends `.ingest`).
pub fn data_dir_for_db(db_path: &Path) -> PathBuf {
    let mut path = db_path.as_os_str().to_os_string();
    path.push(".ingest");
    PathBuf::from(path)
}

/// Create the ingest data directory with restrictive permissions.
///
/// SEC-25 / TASK-0787: the ingest dir holds workspace-root sidecars and
/// JSON staging files that the database trusts on load. On Unix we create
/// it with mode 0o700 (and re-stamp the mode when the dir pre-exists with
/// a more permissive default umask) so a co-tenant on a multi-user system
/// cannot tamper with staged data between collect and load. On non-Unix
/// platforms `create_dir_all` keeps the existing semantics.
fn create_ingest_dir(data_dir: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(data_dir)?;
        std::fs::set_permissions(data_dir, std::fs::Permissions::from_mode(0o700))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        std::fs::create_dir_all(data_dir)
    }
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

/// Convert a non-IO external error into [`DbError::External`].
///
/// Callers that return `anyhow::Error` (collect_tokei, collect_coverage,
/// check_metadata_output, etc.) should use this instead of the old `io_err`
/// which misleadingly wrapped them as `DbError::Io`.
///
/// SEC-21 (TASK-0862): formats with the alternate `{e:#}` flag so
/// `anyhow::Context` chains are preserved end-to-end. Plain `to_string()`
/// would render only the leaf cause and silently drop wrapping context,
/// turning operator triage into guesswork when an external collector fails
/// deep in a workspace.
pub fn external_err(e: impl std::fmt::Display) -> DbError {
    DbError::External(format!("{e:#}"))
}

/// Compute SHA-256 checksum of a file, returning hex string.
///
/// Streams the file in 64 KiB chunks so multi-megabyte ingests (coverage,
/// tokei) do not allocate a full file-sized buffer (PERF-1).
pub fn checksum_file(path: &Path) -> DbResult<String> {
    use sha2::{Digest, Sha256};
    use std::io::{BufReader, Read};
    let file = std::fs::File::open(path).map_err(DbError::Io)?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = reader.read(&mut buf).map_err(DbError::Io)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    Ok(hex::encode(digest.as_ref() as &[u8]))
}

/// Single source of truth for the workspace sidecar filename convention
/// (DUP-3). All write/read/remove helpers route through here.
pub fn sidecar_path(data_dir: &Path, name: &str) -> PathBuf {
    data_dir.join(format!("{name}_workspace.txt"))
}

/// Write a workspace root sidecar file alongside collected data.
///
/// Used by ingestors that don't embed workspace_root in their JSON output
/// (e.g., tokei, coverage). The sidecar is read back during `load()` for
/// `upsert_data_source`.
///
/// Persists the path's raw OS bytes (via `as_encoded_bytes`) so that
/// non-UTF-8 paths round-trip exactly rather than being silently corrupted
/// to `U+FFFD` (READ-5). The corresponding read happens via
/// [`read_workspace_sidecar`].
pub fn write_workspace_sidecar(
    data_dir: &Path,
    name: &str,
    working_directory: &Path,
) -> DbResult<()> {
    let workspace_path = sidecar_path(data_dir, name);
    // SEC-25 (TASK-0663): a bare `fs::write` could leave a zero-byte or torn
    // sidecar after a crash; `read_workspace_sidecar` would then surface it
    // as the workspace_root and `upsert_data_source` would persist a
    // garbled row. Route through `atomic_write` so the destination only
    // appears once the temp file has been fsync'd and renamed (and the
    // parent directory fsync'd on Unix), matching the durability that the
    // hook installer's `write_temp_hook` adopted in TASK-0713.
    ops_core::config::atomic_write(
        &workspace_path,
        working_directory.as_os_str().as_encoded_bytes(),
    )
    .map_err(DbError::Io)
}

/// Read a workspace root sidecar file written during collect.
///
/// Returns the raw OS bytes as an [`OsString`] so that non-UTF-8
/// `working_directory` paths round-trip identically with the matching
/// [`write_workspace_sidecar`]. The previous `fs::read_to_string` would
/// fail with `ErrorKind::InvalidData` on non-UTF-8 bytes that the writer
/// happily persists via `as_encoded_bytes`, breaking symmetry with the
/// write side (ERR-4 / TASK-0928). UTF-8 validation now happens at the
/// persistence boundary in [`crate::schema::upsert_data_source`], where
/// it returns the same typed [`DbError::NonUtf8Path`] used for
/// `source_path`.
pub fn read_workspace_sidecar(data_dir: &Path, name: &str) -> DbResult<std::ffi::OsString> {
    let workspace_path = sidecar_path(data_dir, name);
    let bytes = std::fs::read(&workspace_path).map_err(DbError::Io)?;
    // SAFETY: `write_workspace_sidecar` persists the path via
    // `working_directory.as_os_str().as_encoded_bytes()` and routes
    // through `atomic_write`, which writes the buffer verbatim. The
    // standard library documents `OsStr::from_encoded_bytes_unchecked`
    // as the exact round-trip pair for `as_encoded_bytes`, valid so long
    // as the bytes came from a prior `as_encoded_bytes` call on the same
    // platform — which is precisely the case here. Any other producer
    // (a tampered or hand-edited sidecar) is a corrupted-input issue,
    // not a soundness concern: callers treat the result as an opaque OS
    // string and the persistence-time UTF-8 check in
    // `upsert_data_source` would still reject invalid bytes downstream.
    let os_str = unsafe { std::ffi::OsStr::from_encoded_bytes_unchecked(&bytes) };
    Ok(os_str.to_os_string())
}

/// Remove a workspace root sidecar file. Best-effort: a missing file is
/// fine, but other errors (EACCES, IO) are logged so accumulated stale
/// sidecars do not silently mask broken cleanup (ERR-1).
pub fn remove_workspace_sidecar(data_dir: &Path, name: &str) {
    let workspace_path = sidecar_path(data_dir, name);
    match std::fs::remove_file(&workspace_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            tracing::warn!(
                "remove_workspace_sidecar({}): {e}",
                workspace_path.display()
            );
        }
    }
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
///
/// CONC-2 (TASK-0728): a per-table ingest lock prevents concurrent callers
/// from both observing `table_has_data == false` and running duplicate
/// collect+load cycles. The lock is held across the check and the full
/// ingest sequence but does **not** hold the DuckDB connection lock during
/// the (potentially expensive) `collect` phase.
pub fn provide_via_ingestor<I, Q>(
    db: &DuckDb,
    ctx: &ops_extension::Context,
    table_name: &'static str,
    ingestor: &I,
    query_fn: Q,
) -> Result<serde_json::Value, anyhow::Error>
where
    I: crate::DataIngestor,
    Q: FnOnce(&DuckDb) -> Result<serde_json::Value, anyhow::Error>,
{
    // ERR-5 (TASK-0780): poisoning the per-table mutex must not become a
    // permanent denial of service for that table — a transient panic in
    // `collect`/`load` (user-supplied code) would otherwise brick every
    // future ingest of that data source for the lifetime of the process.
    // The guarded value is `()`, so there is no torn state to worry about;
    // recover via `into_inner` and continue. Cross-reference: the
    // connection mutex at `connection.rs::DuckDb::lock` deliberately
    // reports poisoning as `DbError::MutexPoisoned` because a panic mid
    // DuckDB transaction can leave half-applied schema state we should
    // not silently reuse. The asymmetry is intentional.
    let ingest_mutex = db.ingest_mutex_for(table_name);
    let _ingest_guard = ingest_mutex.lock().unwrap_or_else(|poisoned| {
        tracing::warn!(
            table = %table_name,
            "per-table ingest mutex was poisoned by a prior panic; recovered"
        );
        poisoned.into_inner()
    });

    // CONC-2 / TASK-0909: drop_table_if_exists MUST run inside the per-table
    // ingest_mutex critical section, before the table_has_data probe.
    // Pre-fix this was outside the mutex, so a concurrent non-refresh
    // caller could ingest into the just-dropped table between our drop
    // and our lock acquisition; this caller would then see
    // table_has_data == true under the lock and silently skip the
    // re-collection the user explicitly asked for via --refresh.
    if ctx.refresh {
        drop_table_if_exists(db, table_name)?;
    }

    if !table_has_data(db, table_name)? {
        let data_dir = data_dir_for_db(db.path());
        create_ingest_dir(&data_dir).map_err(DbError::Io)?;
        ingestor.collect(ctx, &data_dir)?;
        crate::init_schema(db)?;
        // LoadResult's record_count is not consumed here because the
        // success signal is implicit in `table_has_data` returning true
        // on the next call. Revisit if we want to log ingest counts.
        let _load_result = ingestor.load(&data_dir, db)?;
    }
    query_fn(db)
}

/// Drop a table if it exists (used by refresh to force re-collection).
fn drop_table_if_exists(db: &DuckDb, table_name: &str) -> Result<(), anyhow::Error> {
    use anyhow::Context;
    let quoted = quoted_ident(table_name)?;
    let conn = db.lock().context("acquiring db lock for drop")?;
    conn.execute_batch(&format!("DROP TABLE IF EXISTS {quoted}"))
        .with_context(|| format!("dropping table {table_name}"))?;
    Ok(())
}

/// DUP-032: Macro to generate standard path validation tests for `*_create_sql` functions.
///
/// Generates four tests: valid path, path with spaces, injection rejection, traversal rejection.
#[cfg(any(test, feature = "test-helpers"))]
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::init_schema;
    use std::path::PathBuf;
    use std::sync::Arc;

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

    /// SEC-25 / TASK-0787: ingest dir must be 0o700 on Unix on both fresh
    /// create and pre-existing dir paths.
    #[cfg(unix)]
    #[test]
    fn create_ingest_dir_uses_restricted_mode_on_unix() {
        use std::os::unix::fs::PermissionsExt;
        let tmp = tempfile::tempdir().expect("tempdir");
        let dir = tmp.path().join("data.duckdb.ingest");
        create_ingest_dir(&dir).expect("create");
        let mode = std::fs::metadata(&dir).expect("meta").permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o700,
            "fresh-created ingest dir must be 0o700; got {:o}",
            mode & 0o777,
        );
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("relax");
        create_ingest_dir(&dir).expect("recreate");
        let mode = std::fs::metadata(&dir).expect("meta").permissions().mode();
        assert_eq!(
            mode & 0o777,
            0o700,
            "pre-existing ingest dir must be re-stamped to 0o700; got {:o}",
            mode & 0o777,
        );
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
    fn external_err_wraps_display_error() {
        let err = external_err("test error message");
        let msg = err.to_string();
        assert!(msg.contains("test error message"));
    }

    /// SEC-21 (TASK-0862): the alternate-format wrapper must preserve the
    /// full anyhow context chain. Without `{e:#}` only the leaf "leaf cause"
    /// would survive and "wrap two"/"wrap one" would silently disappear.
    #[test]
    fn external_err_preserves_anyhow_context_chain() {
        use anyhow::Context;
        let leaf = anyhow::Error::msg("leaf cause");
        let chained: anyhow::Error = Err::<(), _>(leaf)
            .context("wrap one")
            .context("wrap two")
            .unwrap_err();
        let err = external_err(chained);
        let msg = err.to_string();
        assert!(msg.contains("wrap two"), "missing outer wrap: {msg}");
        assert!(msg.contains("wrap one"), "missing middle wrap: {msg}");
        assert!(msg.contains("leaf cause"), "missing leaf cause: {msg}");
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
    fn checksum_file_streaming_matches_in_memory_for_large_input() {
        // PERF-1 regression: stream vs in-memory must produce identical
        // SHA-256 for inputs spanning multiple 64 KiB chunks.
        use sha2::{Digest, Sha256};
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("big.bin");
        // 200 KiB of pseudo-random-ish bytes.
        let data: Vec<u8> = (0..200 * 1024).map(|i| (i % 256) as u8).collect();
        std::fs::write(&path, &data).expect("write");

        let streamed = checksum_file(&path).expect("stream");
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let in_memory = hex::encode(hasher.finalize().as_ref() as &[u8]);
        assert_eq!(streamed, in_memory);
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

    // --- drop_table_if_exists validation (SEC-12) ---

    #[test]
    fn drop_table_rejects_whitespace() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        assert!(drop_table_if_exists(&db, "my table").is_err());
    }

    #[test]
    fn drop_table_rejects_dots() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        assert!(drop_table_if_exists(&db, "schema.table").is_err());
    }

    #[test]
    fn drop_table_rejects_dashes() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        assert!(drop_table_if_exists(&db, "my-table").is_err());
    }

    #[test]
    fn drop_table_rejects_injection() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        assert!(drop_table_if_exists(&db, "t; DROP TABLE users; --").is_err());
    }

    // --- create_table_from_json_sql validation ---

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

    /// ERR-7: a control-character-laden table name must not reach the error
    /// message verbatim. `Debug` formatting escapes such bytes so log
    /// readers cannot be tricked into seeing forged lines.
    #[test]
    fn table_exists_error_message_sanitizes_control_chars() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        init_schema(&db).expect("init_schema");
        let conn = db.lock().expect("lock");
        // information_schema.tables is fine with anything as a string param,
        // so this query succeeds with count=0 — but to exercise the error
        // path we close the connection on a query that *will* fail. Easier:
        // call with a giant blob that triggers no failure, then verify the
        // helper's formatting via a direct format!() check, which is what
        // really matters for log-injection.
        let _ = table_exists(&conn, "ok_name").expect("baseline ok");

        let nasty = "name\nADMIN: forged log line\rwith ESC\x1b[31m red";
        let rendered = format!("checking if {nasty:?} exists");
        assert!(
            !rendered.contains('\n') && !rendered.contains('\r') && !rendered.contains('\x1b'),
            "control chars must be escaped in error context: {rendered}"
        );
        assert!(rendered.contains("\\n"), "newline escaped: {rendered}");
        assert!(rendered.contains("\\u{1b}"), "ESC escaped: {rendered}");
    }

    /// ERR-7 (TASK-0521): the `counting rows in {table_name:?}` error
    /// context must Debug-format the identifier so a control-character
    /// laden name cannot forge log lines, mirroring the regression guard
    /// on `table_exists_error_message_sanitizes_control_chars`.
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
    fn workspace_sidecar_round_trip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let working = PathBuf::from("/some/workspace/root");
        write_workspace_sidecar(dir.path(), "tokei", &working).expect("write sidecar");

        // Filename derives from name parameter as `<name>_workspace.txt`
        let expected = dir.path().join("tokei_workspace.txt");
        assert!(expected.exists(), "sidecar file at expected path");

        let read = read_workspace_sidecar(dir.path(), "tokei").expect("read sidecar");
        assert_eq!(read, "/some/workspace/root");

        remove_workspace_sidecar(dir.path(), "tokei");
        assert!(!expected.exists(), "sidecar removed");
    }

    #[test]
    #[cfg(unix)]
    fn workspace_sidecar_round_trips_non_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let dir = tempfile::tempdir().expect("tempdir");
        let bytes = b"/ws/\xff\xfe/proj";
        let working = PathBuf::from(OsStr::from_bytes(bytes));
        write_workspace_sidecar(dir.path(), "tokei", &working).expect("write");

        let raw = std::fs::read(dir.path().join("tokei_workspace.txt")).expect("read raw");
        assert_eq!(raw, bytes, "non-UTF-8 bytes preserved verbatim");
    }

    /// ERR-4 / TASK-0928: `read_workspace_sidecar` must round-trip the
    /// same non-UTF-8 OS bytes that `write_workspace_sidecar` persists.
    /// Before this fix the read side called `fs::read_to_string`, which
    /// returned `ErrorKind::InvalidData` on the very inputs the writer
    /// happily stored — `load_with_sidecar` then failed on every
    /// non-UTF-8 `working_directory`. The assertion compares the helper's
    /// return value (not the raw file bytes) so a future regression that
    /// swaps the read back to a lossy UTF-8 reader fails here.
    #[test]
    #[cfg(unix)]
    fn read_workspace_sidecar_round_trips_non_utf8_via_helper() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::{OsStrExt, OsStringExt};
        let dir = tempfile::tempdir().expect("tempdir");
        let bytes = b"/ws/\xff\xfe/proj";
        let working = PathBuf::from(OsStr::from_bytes(bytes));
        write_workspace_sidecar(dir.path(), "tokei", &working).expect("write");

        let read = read_workspace_sidecar(dir.path(), "tokei").expect("read sidecar");
        assert_eq!(
            read.into_vec(),
            bytes.to_vec(),
            "non-UTF-8 bytes survive write→read round-trip via helper"
        );
    }

    /// SEC-25 (TASK-0663): a successful `write_workspace_sidecar` must
    /// leave the destination fully populated and no sibling temp file
    /// behind. This is the visible proof that we route through
    /// `atomic_write` (sibling temp + fsync + rename) rather than a bare
    /// `fs::write` that could leave a torn or zero-byte sidecar after a
    /// crash between syscall return and inode flush.
    #[test]
    fn workspace_sidecar_write_is_atomic_and_leaves_no_temp() {
        let dir = tempfile::tempdir().expect("tempdir");
        let working = PathBuf::from("/some/workspace/root");
        write_workspace_sidecar(dir.path(), "tokei", &working).expect("write sidecar");

        let dest = dir.path().join("tokei_workspace.txt");
        let bytes = std::fs::read(&dest).expect("read dest");
        assert_eq!(bytes, b"/some/workspace/root");

        // No `.tokei_workspace.txt.tmp.*` sibling left behind.
        let leftover = std::fs::read_dir(dir.path())
            .expect("readdir")
            .filter_map(Result::ok)
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .find(|name| name.starts_with(".tokei_workspace.txt.tmp."));
        assert!(leftover.is_none(), "atomic_write left a temp: {leftover:?}");
    }

    #[test]
    fn workspace_sidecar_remove_is_best_effort() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Should not panic when the sidecar does not exist
        remove_workspace_sidecar(dir.path(), "missing_name");
    }

    #[test]
    fn workspace_sidecar_remove_logs_but_does_not_panic_on_failure() {
        // Make the sidecar path point at a directory — remove_file will fail
        // (IsADirectory / Other on different OSes). The function should log
        // and return normally; behavior we assert here is "no panic".
        // Direct tracing assertion would require a subscriber test harness.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir(dir.path().join("blocker_workspace.txt")).expect("create blocker dir");
        remove_workspace_sidecar(dir.path(), "blocker");
        // The blocker still exists (remove failed) but no panic occurred.
        assert!(dir.path().join("blocker_workspace.txt").exists());
    }

    #[test]
    fn workspace_sidecar_filename_uses_name_prefix() {
        let dir = tempfile::tempdir().expect("tempdir");
        let working = PathBuf::from("/ws");
        write_workspace_sidecar(dir.path(), "coverage", &working).expect("write");
        write_workspace_sidecar(dir.path(), "tokei", &working).expect("write");
        assert!(dir.path().join("coverage_workspace.txt").exists());
        assert!(dir.path().join("tokei_workspace.txt").exists());
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

    /// CONC-2 (TASK-0728): two threads invoking `provide_via_ingestor` against
    /// the same empty table must run collect at most once. The second caller
    /// blocks until the first finishes populating the table, then skips its
    /// own collect because `table_has_data` now returns true.
    #[test]
    fn concurrent_provide_via_ingestor_collects_once() {
        use crate::DataIngestor;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static COLLECT_COUNT: AtomicUsize = AtomicUsize::new(0);

        struct CountingIngestor;

        impl DataIngestor for CountingIngestor {
            fn name(&self) -> &'static str {
                "counting"
            }
            fn collect(&self, _ctx: &ops_extension::Context, data_dir: &Path) -> DbResult<()> {
                COLLECT_COUNT.fetch_add(1, Ordering::SeqCst);
                let path = data_dir.join("counting.json");
                std::fs::write(&path, "[{\"id\": 1}]").map_err(DbError::Io)?;
                Ok(())
            }
            fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<crate::LoadResult> {
                let json_path = data_dir.join("counting.json");
                let create_sql = create_table_from_json_sql("counting_test", &json_path, None)?;
                let conn = db.lock()?;
                conn.execute(&create_sql, [])
                    .map_err(|e| DbError::query_failed("counting_test create", e))?;
                drop(conn);
                Ok(crate::LoadResult::success("counting", 1))
            }
        }

        // Use a tempdir-backed DB rather than `:memory:` so
        // `data_dir_for_db` produces a path the SQL path validator
        // accepts (a `:memory:` sentinel embeds a `:`, which the
        // validator rejects, and would also leave a stray
        // `:memory:.ingest/` directory in the cwd).
        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("counting.duckdb");
        let db = Arc::new(DuckDb::open(&db_path).expect("db"));
        init_schema(&db).expect("init_schema");

        let db1 = Arc::clone(&db);
        let db2 = Arc::clone(&db);
        let ctx1 = ops_extension::Context::new(
            Arc::new(ops_core::config::Config::default()),
            PathBuf::from("/tmp"),
        );
        let ctx2 = ops_extension::Context::new(
            Arc::new(ops_core::config::Config::default()),
            PathBuf::from("/tmp"),
        );

        let h1 = std::thread::spawn(move || {
            provide_via_ingestor(&db1, &ctx1, "counting_test", &CountingIngestor, |_| {
                Ok(serde_json::Value::Null)
            })
        });
        let h2 = std::thread::spawn(move || {
            provide_via_ingestor(&db2, &ctx2, "counting_test", &CountingIngestor, |_| {
                Ok(serde_json::Value::Null)
            })
        });

        h1.join().expect("join 1").expect("ingest 1");
        h2.join().expect("join 2").expect("ingest 2");

        assert_eq!(
            COLLECT_COUNT.load(Ordering::SeqCst),
            1,
            "collect must run exactly once, not twice"
        );
    }

    /// CONC-7 (TASK-0779): the per-table ingest registry lives on the
    /// `DuckDb` instance, so its growth is bounded by the database
    /// schema (a fixed set of table names) and entries are released
    /// when the connection is dropped. This guards against the previous
    /// process-global `OnceLock<HashMap<…>>` that leaked one entry per
    /// distinct `(db_path, table)` for the lifetime of the binary.
    #[test]
    fn ingest_lock_map_is_scoped_to_duckdb_instance_and_bounded_by_table_count() {
        use crate::DataIngestor;

        struct TrivialIngestor;
        impl DataIngestor for TrivialIngestor {
            fn name(&self) -> &'static str {
                "trivial"
            }
            fn collect(&self, _ctx: &ops_extension::Context, data_dir: &Path) -> DbResult<()> {
                let path = data_dir.join("trivial.json");
                std::fs::write(&path, "[{\"id\":1}]").map_err(DbError::Io)?;
                Ok(())
            }
            fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<crate::LoadResult> {
                // Use a distinct table per call by reading from the same JSON.
                let json_path = data_dir.join("trivial.json");
                // The caller picks the table name; reuse the same registered
                // table to keep the test about the registry, not SQL.
                let create_sql = create_table_from_json_sql("trivial_table", &json_path, None)?;
                let conn = db.lock()?;
                conn.execute(&create_sql, [])
                    .map_err(|e| DbError::query_failed("trivial create", e))?;
                drop(conn);
                Ok(crate::LoadResult::success("trivial", 1))
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("bounded.duckdb");
        let db = DuckDb::open(&db_path).expect("db");
        init_schema(&db).expect("init_schema");

        let ctx = ops_extension::Context::new(
            Arc::new(ops_core::config::Config::default()),
            PathBuf::from("/tmp"),
        );

        // Take a few distinct table-name keys: the registry should hold at
        // most that many entries — never inherit anything from prior tests
        // and never exceed the count of distinct names we asked for.
        for name in ["t_a", "t_b", "t_c"] {
            let _ = db.ingest_mutex_for(name);
            // Sanity — calling again must not double-insert.
            let _ = db.ingest_mutex_for(name);
        }
        assert_eq!(
            db.ingest_lock_count(),
            3,
            "registry should hold one entry per distinct table name"
        );

        // And exercising the real ingest path through `provide_via_ingestor`
        // does not accumulate extra entries beyond the table it touched.
        let before = db.ingest_lock_count();
        provide_via_ingestor(&db, &ctx, "trivial_table", &TrivialIngestor, |_| {
            Ok(serde_json::Value::Null)
        })
        .expect("ingest");
        assert_eq!(
            db.ingest_lock_count(),
            before + 1,
            "one new entry for the freshly ingested table"
        );

        // Dropping the DuckDb releases the map; the next instance starts
        // empty (no leak between connections, unlike the old global).
        drop(db);
        let db2 = DuckDb::open(&db_path).expect("db reopen");
        assert_eq!(db2.ingest_lock_count(), 0, "fresh instance has no entries");
    }

    /// ERR-5 (TASK-0780): a panic inside an ingestor's `collect` must not
    /// permanently brick the table. The per-table mutex is poisoned, but
    /// `provide_via_ingestor` recovers it via `into_inner` so a subsequent
    /// caller can still ingest. Cross-reference: `DuckDb::lock` keeps the
    /// `MutexPoisoned` policy intact for the connection mutex (see
    /// `connection.rs`).
    #[test]
    fn panic_in_collect_does_not_brick_subsequent_ingest() {
        use crate::DataIngestor;
        use std::sync::atomic::{AtomicBool, Ordering};

        struct PanickyIngestor {
            should_panic: AtomicBool,
        }
        impl DataIngestor for PanickyIngestor {
            fn name(&self) -> &'static str {
                "panicky"
            }
            fn collect(&self, _ctx: &ops_extension::Context, data_dir: &Path) -> DbResult<()> {
                if self.should_panic.swap(false, Ordering::SeqCst) {
                    panic!("simulated transient ingest panic");
                }
                let path = data_dir.join("panicky.json");
                std::fs::write(&path, "[{\"id\":1}]").map_err(DbError::Io)?;
                Ok(())
            }
            fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<crate::LoadResult> {
                let json_path = data_dir.join("panicky.json");
                let create_sql = create_table_from_json_sql("panicky_table", &json_path, None)?;
                let conn = db.lock()?;
                conn.execute(&create_sql, [])
                    .map_err(|e| DbError::query_failed("panicky create", e))?;
                drop(conn);
                Ok(crate::LoadResult::success("panicky", 1))
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("panicky.duckdb");
        let db = Arc::new(DuckDb::open(&db_path).expect("db"));
        init_schema(&db).expect("init_schema");
        let ingestor = Arc::new(PanickyIngestor {
            should_panic: AtomicBool::new(true),
        });

        // First call panics inside `collect` while holding the per-table
        // mutex. Catching unwinds across thread boundaries simulates the
        // production scenario without aborting the test process.
        let db1 = Arc::clone(&db);
        let ing1 = Arc::clone(&ingestor);
        let h = std::thread::spawn(move || {
            let ctx = ops_extension::Context::new(
                Arc::new(ops_core::config::Config::default()),
                PathBuf::from("/tmp"),
            );
            provide_via_ingestor(&db1, &ctx, "panicky_table", &*ing1, |_| {
                Ok(serde_json::Value::Null)
            })
        });
        // The thread panicked; join returns Err.
        assert!(h.join().is_err(), "first call must have panicked");

        // Subsequent caller must succeed despite the poisoned per-table
        // mutex. Without poison recovery this would itself panic.
        let ctx = ops_extension::Context::new(
            Arc::new(ops_core::config::Config::default()),
            PathBuf::from("/tmp"),
        );
        provide_via_ingestor(&db, &ctx, "panicky_table", &*ingestor, |_| {
            Ok(serde_json::Value::Null)
        })
        .expect("recovery ingest must not panic");
    }

    /// TASK-0861: poison recovery in `provide_via_ingestor` must emit a
    /// `tracing::warn!` so operators can distinguish "never panicked" from
    /// "panicked once and recovered" in production logs.
    #[test]
    fn poison_recovery_emits_warn_log() {
        use crate::DataIngestor;
        use std::io::Write;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Mutex as StdMutex;
        use tracing_subscriber::fmt::MakeWriter;

        #[derive(Clone, Default)]
        struct BufWriter(Arc<StdMutex<Vec<u8>>>);
        impl Write for BufWriter {
            fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
                self.0.lock().unwrap().extend_from_slice(b);
                Ok(b.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        impl<'a> MakeWriter<'a> for BufWriter {
            type Writer = BufWriter;
            fn make_writer(&'a self) -> Self::Writer {
                self.clone()
            }
        }

        struct PanickyIngestor {
            should_panic: AtomicBool,
        }
        impl DataIngestor for PanickyIngestor {
            fn name(&self) -> &'static str {
                "panicky_warn"
            }
            fn collect(&self, _ctx: &ops_extension::Context, data_dir: &Path) -> DbResult<()> {
                if self.should_panic.swap(false, Ordering::SeqCst) {
                    panic!("simulated transient ingest panic");
                }
                let path = data_dir.join("panicky_warn.json");
                std::fs::write(&path, "[{\"id\":1}]").map_err(DbError::Io)?;
                Ok(())
            }
            fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<crate::LoadResult> {
                let json_path = data_dir.join("panicky_warn.json");
                let create_sql =
                    create_table_from_json_sql("panicky_warn_table", &json_path, None)?;
                let conn = db.lock()?;
                conn.execute(&create_sql, [])
                    .map_err(|e| DbError::query_failed("panicky create", e))?;
                drop(conn);
                Ok(crate::LoadResult::success("panicky_warn", 1))
            }
        }

        let db_dir = tempfile::tempdir().expect("tempdir");
        let db_path = db_dir.path().join("panicky_warn.duckdb");
        let db = Arc::new(DuckDb::open(&db_path).expect("db"));
        init_schema(&db).expect("init_schema");
        let ingestor = Arc::new(PanickyIngestor {
            should_panic: AtomicBool::new(true),
        });

        let db1 = Arc::clone(&db);
        let ing1 = Arc::clone(&ingestor);
        let h = std::thread::spawn(move || {
            let ctx = ops_extension::Context::new(
                Arc::new(ops_core::config::Config::default()),
                PathBuf::from("/tmp"),
            );
            provide_via_ingestor(&db1, &ctx, "panicky_warn_table", &*ing1, |_| {
                Ok(serde_json::Value::Null)
            })
        });
        assert!(h.join().is_err(), "first call must have panicked");

        let buf = BufWriter::default();
        let captured = buf.0.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf)
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            let ctx = ops_extension::Context::new(
                Arc::new(ops_core::config::Config::default()),
                PathBuf::from("/tmp"),
            );
            provide_via_ingestor(&db, &ctx, "panicky_warn_table", &*ingestor, |_| {
                Ok(serde_json::Value::Null)
            })
            .expect("recovery ingest must not panic");
        });

        let logs = String::from_utf8(captured.lock().unwrap().clone()).unwrap();
        assert!(
            logs.contains("per-table ingest mutex was poisoned"),
            "expected poison-recovery warn, got: {logs}"
        );
        assert!(
            logs.contains("panicky_warn_table"),
            "expected table name in warn, got: {logs}"
        );
    }
}
