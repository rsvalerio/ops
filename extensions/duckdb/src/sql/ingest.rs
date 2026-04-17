//! Table creation, sidecar I/O, and data pipeline helpers.

use crate::{DbError, DbResult, DuckDb};
use std::path::{Path, PathBuf};

use super::validation::*;

/// Generate `CREATE OR REPLACE TABLE <name> AS SELECT * FROM read_json_auto(...)` SQL (DUP-009).
///
/// Validates and escapes the path for safe interpolation. Pass `extra_opts` for
/// additional read_json_auto parameters (e.g., `"maximum_object_size=67108864"`).
pub fn create_table_from_json_sql(
    table_name: &str,
    path: &Path,
    extra_opts: Option<&str>,
) -> Result<String, SqlError> {
    validate_identifier(table_name)?;
    let escaped = prepare_path_for_sql(path)?;
    match extra_opts {
        Some(opts) => Ok(format!(
            "CREATE OR REPLACE TABLE {table_name} AS SELECT * FROM read_json_auto('{escaped}', {opts})",
        )),
        None => Ok(format!(
            "CREATE OR REPLACE TABLE {table_name} AS SELECT * FROM read_json_auto('{escaped}')",
        )),
    }
}

/// Check if a table or view exists in the database.
pub(super) fn table_exists(
    conn: &duckdb::Connection,
    table_name: &str,
) -> Result<bool, anyhow::Error> {
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
    validate_identifier(table_name)?;
    let row_count: i64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM \"{table_name}\""),
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
    validate_identifier(table_name)?;
    let conn = db.lock().context("acquiring db lock for drop")?;
    conn.execute_batch(&format!("DROP TABLE IF EXISTS \"{table_name}\""))
        .with_context(|| format!("dropping table {table_name}"))?;
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
}
