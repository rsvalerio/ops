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
            record_count   INTEGER NOT NULL DEFAULT 0,
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

/// Metadata describing a loaded data source row.
pub struct DataSourceMetadata<'a> {
    pub source_name: &'a str,
    pub workspace_root: &'a str,
    pub source_path: &'a Path,
    pub record_count: u64,
    pub checksum: &'a str,
}

/// Upsert a data_sources row after a load.
///
/// Fails fast with [`DbError::NonUtf8Path`] when `source_path` is not valid
/// UTF-8 — the previous lossy conversion silently stored a string that
/// could not be mapped back to the actual file (ERR-4).
pub fn upsert_data_source(db: &DuckDb, meta: &DataSourceMetadata<'_>) -> DbResult<()> {
    let path_str = meta
        .source_path
        .to_str()
        .ok_or_else(|| DbError::NonUtf8Path(meta.source_path.as_os_str().to_os_string()))?;
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
            meta.workspace_root,
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
            &DataSourceMetadata {
                source_name: "metadata",
                workspace_root: "/ws",
                source_path: bad_path,
                record_count: 1,
                checksum: "abc",
            },
        );
        assert!(matches!(result, Err(DbError::NonUtf8Path(_))));
    }

    #[test]
    fn upsert_and_get_source_checksum() {
        let db = DuckDb::open_in_memory().unwrap();
        init_schema(&db).unwrap();
        upsert_data_source(
            &db,
            &DataSourceMetadata {
                source_name: "metadata",
                workspace_root: "/ws",
                source_path: Path::new("/ws/target/ops/metadata.json"),
                record_count: 1,
                checksum: "abc123",
            },
        )
        .unwrap();
        let c = get_source_checksum(&db, "metadata", "/ws").unwrap();
        assert_eq!(c.as_deref(), Some("abc123"));
    }
}
