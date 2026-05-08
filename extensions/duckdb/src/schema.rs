//! Schema initialization and tracking for DuckDb.

use crate::connection::DuckDb;
use crate::error::{DbError, DbResult};
use std::path::Path;

/// Create the data_sources tracking table if it does not exist.
pub fn init_schema(db: &DuckDb) -> DbResult<()> {
    let conn = db.lock()?;
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS data_sources (
            source_name    VARCHAR NOT NULL,
            workspace_root VARCHAR NOT NULL,
            loaded_at      TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
            source_path    VARCHAR NOT NULL,
            record_count   BIGINT NOT NULL DEFAULT 0,
            checksum       VARCHAR(64) NOT NULL,
            metadata       JSON,
            PRIMARY KEY (source_name, workspace_root)
        );
        "#,
    )
    .map_err(|e| DbError::query_failed("init_schema", e))?;
    Ok(())
}

/// Get stored checksum for a source and workspace, if any.
#[allow(dead_code)]
pub fn get_source_checksum(
    db: &DuckDb,
    source_name: &str,
    workspace_root: &str,
) -> DbResult<Option<String>> {
    let conn = db.lock()?;
    let mut stmt = conn
        .prepare("SELECT checksum FROM data_sources WHERE source_name = ? AND workspace_root = ?")
        .map_err(|e| DbError::query_failed("get_source_checksum", e))?;
    let row = stmt.query_row(duckdb::params![source_name, workspace_root], |r| {
        r.get::<_, String>(0)
    });
    match row {
        Ok(s) => Ok(Some(s)),
        Err(duckdb::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(DbError::DuckDb(e)),
    }
}

/// API-2 / TASK-0912: distinct newtypes for the two adjacent `&str`
/// parameters of [`DataSourceMetadata::new`]. Both halves of the
/// `(source_name, workspace_root)` primary key were silently swappable
/// before; a swap silently wrote rows under the wrong key, producing
/// duplicate ingest records and divergent checksums no future run could
/// reconcile. Swap is now a compile error.
#[derive(Debug, Clone, Copy)]
pub struct SourceName<'a>(pub &'a str);

#[derive(Debug, Clone, Copy)]
pub struct WorkspaceRoot<'a>(pub &'a std::ffi::OsStr);

/// Metadata describing a loaded data source row.
#[non_exhaustive]
pub struct DataSourceMetadata<'a> {
    pub source_name: &'a str,
    pub workspace_root: &'a std::ffi::OsStr,
    pub source_path: &'a Path,
    pub record_count: u64,
    pub checksum: &'a str,
}

impl<'a> DataSourceMetadata<'a> {
    pub fn new(
        source_name: SourceName<'a>,
        workspace_root: WorkspaceRoot<'a>,
        source_path: &'a Path,
        record_count: u64,
        checksum: &'a str,
    ) -> Self {
        Self {
            source_name: source_name.0,
            workspace_root: workspace_root.0,
            source_path,
            record_count,
            checksum,
        }
    }
}

/// Upsert a data_sources row after a load.
///
/// Fails fast with [`DbError::NonUtf8Path`] when `source_path` is not valid
/// UTF-8 — the previous lossy conversion silently stored a string that
/// could not be mapped back to the actual file (ERR-4).
///
/// ERR-1 / TASK-1103: the same fail-fast contract is mirrored in
/// `ops_about::identity::build_identity_value`, which rejects a non-UTF-8
/// `cwd` with a typed [`ops_extension::DataProviderError`] instead of
/// shipping `U+FFFD`-mangled bytes into the `project_root` JSON field.
/// Any path persisted into a downstream consumer (this DuckDB row, the
/// `ProjectIdentity` JSON, audit logs) must round-trip faithfully — so
/// the two callsites share one policy: typed error on non-UTF-8, no
/// lossy `Path::display` / `to_string_lossy` shortcut.
pub fn upsert_data_source(db: &DuckDb, meta: &DataSourceMetadata<'_>) -> DbResult<()> {
    let path_str = meta
        .source_path
        .to_str()
        .ok_or_else(|| DbError::NonUtf8Path(meta.source_path.as_os_str().to_os_string()))?;
    // ERR-4 / TASK-0928: now that `read_workspace_sidecar` preserves raw
    // OS bytes verbatim (matching the writer's `as_encoded_bytes`), reject
    // a non-UTF-8 workspace_root with the same typed error used for
    // `source_path` rather than letting a lossy `to_string_lossy` ship a
    // garbled key into the `(source_name, workspace_root)` PK.
    let workspace_root_str = meta
        .workspace_root
        .to_str()
        .ok_or_else(|| DbError::NonUtf8Path(meta.workspace_root.to_os_string()))?;
    let record_count_i64 = i64::try_from(meta.record_count)
        .map_err(|_| DbError::RecordCountOverflow(meta.record_count))?;
    let conn = db.lock()?;
    conn.execute(
        r#"
        INSERT INTO data_sources (source_name, workspace_root, source_path, record_count, checksum)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT (source_name, workspace_root) DO UPDATE SET
            loaded_at = get_current_timestamp(),
            source_path = excluded.source_path,
            record_count = excluded.record_count,
            checksum = excluded.checksum
        "#,
        duckdb::params![
            meta.source_name,
            workspace_root_str,
            path_str,
            record_count_i64,
            meta.checksum
        ],
    )
    .map_err(|e| DbError::query_failed("upsert_data_source", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::connection::DuckDb;
    use std::path::Path;

    #[test]
    fn init_schema_creates_data_sources() {
        let db = DuckDb::open_in_memory().unwrap();
        init_schema(&db).unwrap();
        let conn = db.lock().unwrap();
        conn.execute("SELECT 1 FROM data_sources LIMIT 0", [])
            .unwrap();
    }

    #[test]
    fn get_source_checksum_none_when_empty() {
        let db = DuckDb::open_in_memory().unwrap();
        init_schema(&db).unwrap();
        let c = get_source_checksum(&db, "metadata", "/ws").unwrap();
        assert!(c.is_none());
    }

    #[test]
    #[cfg(unix)]
    fn upsert_data_source_rejects_non_utf8_path() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let db = DuckDb::open_in_memory().unwrap();
        init_schema(&db).unwrap();
        let bytes = b"/ws/\xff\xfe.json";
        let bad_path = std::path::Path::new(OsStr::from_bytes(bytes));
        let result = upsert_data_source(
            &db,
            &DataSourceMetadata::new(
                SourceName("metadata"),
                WorkspaceRoot(std::ffi::OsStr::new("/ws")),
                bad_path,
                1,
                "abc",
            ),
        );
        assert!(matches!(result, Err(DbError::NonUtf8Path(_))));
    }

    /// ERR-1 (TASK-0885): the column was widened from INTEGER (i32) to
    /// BIGINT (i64) so counts exceeding `i32::MAX` round-trip without
    /// truncation or driver-level bind error.
    #[test]
    fn record_count_over_i32_max_round_trips() {
        let db = DuckDb::open_in_memory().unwrap();
        init_schema(&db).unwrap();
        let big = (i32::MAX as u64) + 7;
        upsert_data_source(
            &db,
            &DataSourceMetadata::new(
                SourceName("big"),
                WorkspaceRoot(std::ffi::OsStr::new("/ws")),
                Path::new("/ws/target/ops/big.json"),
                big,
                "abc",
            ),
        )
        .unwrap();
        let conn = db.lock().unwrap();
        let stored: i64 = conn
            .query_row(
                "SELECT record_count FROM data_sources WHERE source_name = ? AND workspace_root = ?",
                duckdb::params!["big", "/ws"],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored as u64, big);
    }

    #[test]
    fn upsert_and_get_source_checksum() {
        let db = DuckDb::open_in_memory().unwrap();
        init_schema(&db).unwrap();
        upsert_data_source(
            &db,
            &DataSourceMetadata::new(
                SourceName("metadata"),
                WorkspaceRoot(std::ffi::OsStr::new("/ws")),
                Path::new("/ws/target/ops/metadata.json"),
                1,
                "abc123",
            ),
        )
        .unwrap();
        let c = get_source_checksum(&db, "metadata", "/ws").unwrap();
        assert_eq!(c.as_deref(), Some("abc123"));
    }
}
