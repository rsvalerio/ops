//! MetadataIngestor: collect cargo metadata and load into DuckDB.

use crate::views;
use crate::{check_metadata_output, run_cargo_metadata};
use ops_duckdb::sql::io_err;
use ops_duckdb::{
    init_schema, upsert_data_source, DataIngestor, DbError, DbResult, DuckDb, LoadResult,
};
use ops_extension::Context;
use std::path::Path;

pub struct MetadataIngestor;

impl DataIngestor for MetadataIngestor {
    fn name(&self) -> &'static str {
        "metadata"
    }

    fn collect(&self, ctx: &Context, data_dir: &Path) -> DbResult<()> {
        std::fs::create_dir_all(data_dir).map_err(DbError::Io)?;
        let output = run_cargo_metadata(&ctx.working_directory).map_err(|e| match e {
            ops_core::subprocess::RunError::Io(io) => DbError::Io(io),
            ops_core::subprocess::RunError::Timeout(t) => DbError::Timeout {
                label: t.label,
                timeout_secs: t.timeout.as_secs(),
            },
        })?;
        check_metadata_output(&output).map_err(io_err)?;
        let path = data_dir.join("metadata.json");
        std::fs::write(&path, &output.stdout).map_err(DbError::Io)?;
        Ok(())
    }

    fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<LoadResult> {
        init_schema(db)?;
        let conn = db.lock()?;

        let path = data_dir.join("metadata.json");
        let sql = views::metadata_raw_create_sql(&path).map_err(io_err)?;
        conn.execute(&sql, [])
            .map_err(|e| DbError::query_failed("metadata_raw create", e))?;

        let view_sql = views::crate_dependencies_view_sql();
        conn.execute(&view_sql, [])
            .map_err(|e| DbError::query_failed("crate_dependencies view", e))?;

        let workspace_root: String = conn
            .query_row(
                "SELECT workspace_root FROM metadata_raw LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| DbError::query_failed("metadata_raw workspace_root extract", e))?;

        drop(conn);

        let record_count = 1u64;
        let checksum = ops_duckdb::sql::checksum_file(&data_dir.join("metadata.json"))?;
        upsert_data_source(
            db,
            &ops_duckdb::DataSourceMetadata {
                source_name: self.name(),
                workspace_root: &workspace_root,
                source_path: &path,
                record_count,
                checksum: &checksum,
            },
        )?;

        // TASK-0510: cleanup is best-effort. The DuckDB row is already
        // committed; failing the whole load over a remove_file error
        // (read-only mount, AV race) makes subsequent invocations think
        // ingestion is incomplete and retry it. Log at warn so the leftover
        // file is observable, but do not propagate.
        if let Err(e) = std::fs::remove_file(&path) {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "failed to remove staged metadata file after successful load; leaving in place"
            );
        }

        Ok(LoadResult::success(self.name(), record_count))
    }

    fn checksum(&self, data_dir: &Path) -> DbResult<String> {
        ops_duckdb::sql::checksum_file(&data_dir.join("metadata.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_ingestor_name() {
        let ingestor = MetadataIngestor;
        assert_eq!(ingestor.name(), "metadata");
    }

    #[test]
    fn metadata_collect_fails_with_nonexistent_directory() {
        let ingestor = MetadataIngestor;
        // Build a path that is guaranteed not to exist by joining onto a
        // tempdir we never populate; the tempdir itself exists, but the
        // sub-path inside it does not.
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let ctx = ops_extension::Context::test_context(missing);
        let data_dir = tempfile::tempdir().unwrap();
        let result = ingestor.collect(&ctx, data_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn metadata_checksum_fails_when_file_missing() {
        let data_dir = tempfile::tempdir().unwrap();
        let ingestor = MetadataIngestor;
        let result = ingestor.checksum(data_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn metadata_load_with_sample_data() {
        let data_dir = tempfile::tempdir().unwrap();

        // Write a minimal cargo metadata JSON file.
        // All nullable string fields use explicit strings so DuckDB schema inference
        // picks up the correct types (null-only columns get inferred as integers).
        let metadata_json = serde_json::json!({
            "packages": [{
                "name": "test-crate",
                "version": "0.1.0",
                "id": "test-crate 0.1.0 (path+file:///test)",
                "source": "registry+https://github.com/rust-lang/crates.io-index",
                "dependencies": [{
                    "name": "serde",
                    "source": "registry+https://github.com/rust-lang/crates.io-index",
                    "req": "^1.0",
                    "kind": "normal",
                    "optional": false,
                    "uses_default_features": true,
                    "features": [],
                    "target": "",
                    "rename": "",
                    "registry": ""
                }],
                "targets": [],
                "features": {},
                "manifest_path": "/test/Cargo.toml",
                "metadata": {},
                "publish": [],
                "authors": [],
                "categories": [],
                "keywords": [],
                "readme": "",
                "repository": "",
                "homepage": "",
                "documentation": "",
                "edition": "2021",
                "links": "",
                "default_run": "",
                "rust_version": "",
                "license": "",
                "license_file": "",
                "description": ""
            }],
            "workspace_members": [
                "test-crate 0.1.0 (path+file:///test)"
            ],
            "workspace_default_members": [
                "test-crate 0.1.0 (path+file:///test)"
            ],
            "resolve": {"nodes": [], "root": ""},
            "target_directory": "/test/target",
            "version": 1,
            "workspace_root": "/test",
            "metadata": {}
        });
        let json_path = data_dir.path().join("metadata.json");
        std::fs::write(
            &json_path,
            serde_json::to_vec_pretty(&metadata_json).unwrap(),
        )
        .unwrap();

        let db = DuckDb::open_in_memory().expect("open in-memory db");
        let ingestor = MetadataIngestor;
        let result = ingestor.load(data_dir.path(), &db);
        assert!(result.is_ok());
        let load_result = result.unwrap();
        assert_eq!(load_result.source_name, "metadata");
        assert_eq!(load_result.record_count, 1);

        // Verify the view was created
        let conn = db.lock().unwrap();
        let dep_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM crate_dependencies WHERE dependency_name = 'serde'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(dep_count, 1);

        // Verify JSON file was cleaned up
        assert!(!json_path.exists());
    }
}
