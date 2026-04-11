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

/// Upsert a data_sources row after a load.
#[allow(clippy::too_many_arguments)]
pub fn upsert_data_source(
    db: &DuckDb,
    source_name: &str,
    workspace_root: &str,
    source_path: &Path,
    record_count: u64,
    checksum: &str,
) -> DbResult<()> {
    let path_str = source_path.to_string_lossy();
    let record_count_i64 =
        i64::try_from(record_count).map_err(|_| DbError::RecordCountOverflow(record_count))?;
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
            source_name,
            workspace_root,
            path_str.as_ref(),
            record_count_i64,
            checksum
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
    fn upsert_and_get_source_checksum() {
        let db = DuckDb::open_in_memory().unwrap();
        init_schema(&db).unwrap();
        upsert_data_source(
            &db,
            "metadata",
            "/ws",
            Path::new("/ws/target/ops/metadata.json"),
            1,
            "abc123",
        )
        .unwrap();
        let c = get_source_checksum(&db, "metadata", "/ws").unwrap();
        assert_eq!(c.as_deref(), Some("abc123"));
    }
}
