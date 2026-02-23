//! MetadataIngestor: collect cargo metadata and load into DuckDB.

use crate::extension::Context;
use crate::extensions::metadata::views;
use crate::extensions::metadata::{check_metadata_output, run_cargo_metadata};
use crate::extensions::ops_db::{
    init_schema, upsert_data_source, DataIngestor, DbError, DbResult, LoadResult, OpsDb,
};
use std::path::Path;

pub struct MetadataIngestor;

fn io_err<E: std::fmt::Display>(e: E) -> DbError {
    DbError::Io(std::io::Error::other(e.to_string()))
}

impl DataIngestor for MetadataIngestor {
    fn name(&self) -> &'static str {
        "metadata"
    }

    fn collect(&self, ctx: &Context, data_dir: &Path) -> DbResult<()> {
        std::fs::create_dir_all(data_dir).map_err(DbError::Io)?;
        let output = run_cargo_metadata(&ctx.working_directory).map_err(DbError::Io)?;
        check_metadata_output(&output).map_err(io_err)?;
        let path = data_dir.join("metadata.json");
        std::fs::write(&path, &output.stdout).map_err(DbError::Io)?;
        Ok(())
    }

    fn load(&self, data_dir: &Path, db: &OpsDb) -> DbResult<LoadResult> {
        init_schema(db)?;
        let conn = db.lock()?;

        let path = data_dir.join("metadata.json");
        let sql = views::metadata_raw_create_sql(&path).map_err(io_err)?;
        conn.execute(&sql, [])
            .map_err(|e| DbError::query_failed("metadata_raw create", e))?;

        let workspace_root: String = conn
            .query_row(
                "SELECT workspace_root FROM metadata_raw LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| DbError::query_failed("metadata_raw workspace_root extract", e))?;

        drop(conn);

        let record_count = 1u64;
        let checksum = Self::checksum_static(data_dir)?;
        upsert_data_source(
            db,
            "metadata",
            &workspace_root,
            &path,
            record_count,
            &checksum,
        )?;

        // Note: File is deleted after successful load. If the load fails before this point,
        // the staged file remains and can be re-loaded. This is intentional - it allows
        // recovery from transient failures without re-running cargo metadata.
        std::fs::remove_file(&path).map_err(DbError::Io)?;

        Ok(LoadResult::success("metadata", record_count))
    }

    fn checksum(&self, data_dir: &Path) -> DbResult<String> {
        Self::checksum_static(data_dir)
    }
}

impl MetadataIngestor {
    fn checksum_static(data_dir: &Path) -> DbResult<String> {
        use sha2::{Digest, Sha256};
        let path = data_dir.join("metadata.json");
        let data = std::fs::read(&path).map_err(crate::extensions::ops_db::DbError::Io)?;
        let mut hasher = Sha256::new();
        hasher.update(&data);
        let digest = hasher.finalize();
        Ok(hex::encode(digest.as_ref() as &[u8]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// TQ-002: Test checksum_static with valid data.
    #[test]
    fn checksum_static_returns_sha256_hex() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("metadata.json"),
            r#"{"workspace_root": "/test"}"#,
        )
        .expect("write");

        let checksum = MetadataIngestor::checksum_static(dir.path()).expect("checksum");
        assert_eq!(checksum.len(), 64, "SHA-256 hex should be 64 chars");
        assert!(checksum.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// TQ-002: Test checksum_static fails when file is missing.
    #[test]
    fn checksum_static_fails_when_file_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = MetadataIngestor::checksum_static(dir.path());
        assert!(result.is_err(), "should fail for missing file");
    }

    /// TQ-002: Test checksum_static is deterministic.
    #[test]
    fn checksum_static_is_deterministic() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("metadata.json"), b"test data").expect("write");

        let c1 = MetadataIngestor::checksum_static(dir.path()).expect("checksum1");
        let c2 = MetadataIngestor::checksum_static(dir.path()).expect("checksum2");
        assert_eq!(c1, c2, "checksum should be deterministic");
    }

    /// TQ-002: Test io_err creates proper DbError.
    #[test]
    fn io_err_wraps_display_error() {
        let err = io_err("test error message");
        let msg = err.to_string();
        assert!(msg.contains("test error message"));
    }

    /// TQ-002: Test MetadataIngestor name method.
    #[test]
    fn metadata_ingestor_name() {
        let ingestor = MetadataIngestor;
        assert_eq!(ingestor.name(), "metadata");
    }
}
