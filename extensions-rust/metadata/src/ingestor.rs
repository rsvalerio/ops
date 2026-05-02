//! MetadataIngestor: collect cargo metadata and load into DuckDB.

use crate::views;
use crate::{check_metadata_output, run_cargo_metadata};
use ops_duckdb::sql::external_err;
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
            other => DbError::External(format!("cargo metadata: {other}")),
        })?;
        check_metadata_output(&output).map_err(external_err)?;
        let path = data_dir.join("metadata.json");
        // SEC-25 / TASK-0933: persist `cargo metadata` stdout via
        // `ops_core::config::atomic_write` (sibling temp + fsync + rename),
        // matching the TASK-0911 fix for `SidecarIngestorConfig::collect_sidecar`.
        // A crash mid-write previously left a torn or zero-byte
        // `metadata.json` that the subsequent `load` step would feed to
        // DuckDB's `read_json_auto`, corrupting the database with truncated
        // input. With `atomic_write` the destination either holds the previous
        // payload or the full new payload — never a partial write.
        ops_core::config::atomic_write(&path, &output.stdout).map_err(DbError::Io)?;
        Ok(())
    }

    fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<LoadResult> {
        init_schema(db)?;
        let conn = db.lock()?;

        let path = data_dir.join("metadata.json");
        let sql = views::metadata_raw_create_sql(&path)?;
        conn.execute(&sql, [])
            .map_err(|e| DbError::query_failed("metadata_raw create", e))?;

        let view_sql = views::crate_dependencies_view_sql();
        conn.execute(&view_sql, [])
            .map_err(|e| DbError::query_failed("crate_dependencies view", e))?;

        // API-1 (TASK-0606): record_count is what `data_sources` exposes to
        // downstream tooling as a health signal. Query it instead of hard-
        // coding 1 — today the `metadata_raw` JSON ingest produces a single
        // row per workspace, but a future schema variant (multi-target,
        // workspace-of-workspaces) could yield more, and a hard-coded 1
        // would silently misreport that.
        let record_count: u64 = conn
            .query_row("SELECT count(*) FROM metadata_raw", [], |row| {
                row.get::<_, i64>(0)
            })
            .map_err(|e| DbError::query_failed("metadata_raw count", e))
            .and_then(|raw| {
                u64::try_from(raw).map_err(|_| DbError::InvalidRecordCount {
                    table: "metadata_raw".to_string(),
                    count: raw,
                })
            })?;

        let workspace_root: String = conn
            .query_row(
                "SELECT workspace_root FROM metadata_raw ORDER BY rowid LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| {
                // READ-5 (TASK-0614): when DuckDB infers `workspace_root`
                // as null or non-VARCHAR (cargo metadata edge case), the
                // raw error names neither the observed type nor the
                // offending value. Probe the column type via `typeof(...)`
                // so the operator sees what shape DuckDB actually saw —
                // probe failures fall back to a static label so the error
                // is at worst as informative as before.
                let observed_type = conn
                    .query_row(
                        "SELECT typeof(workspace_root) FROM metadata_raw ORDER BY rowid LIMIT 1",
                        [],
                        |row| row.get::<_, String>(0),
                    )
                    .unwrap_or_else(|_| "<probe failed>".to_string());
                DbError::query_failed(
                    format!("metadata_raw workspace_root extract (observed type: {observed_type})"),
                    e,
                )
            })?;

        drop(conn);

        let checksum = ops_duckdb::sql::checksum_file(&data_dir.join("metadata.json"))?;
        upsert_data_source(
            db,
            &ops_duckdb::DataSourceMetadata::new(
                ops_duckdb::SourceName(self.name()),
                ops_duckdb::WorkspaceRoot(std::ffi::OsStr::new(&workspace_root)),
                &path,
                record_count,
                &checksum,
            ),
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

    /// SEC-25 / TASK-0933: a successful `MetadataIngestor::collect` must
    /// leave no `.tmp.*` leftover from the `atomic_write` sibling-temp
    /// pattern. Pin the cargo-metadata stdout write on the same crash-safe
    /// helper that `SidecarIngestorConfig::collect_sidecar` uses (TASK-0911),
    /// so a crash mid-write leaves either no `metadata.json` or the previous
    /// version — never a partial.
    #[test]
    fn metadata_collect_writes_atomically_no_tmp_leftover() {
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let ctx = ops_extension::Context::test_context(manifest_dir);
        let data_dir = tempfile::tempdir().expect("tempdir");
        let ingestor = MetadataIngestor;
        ingestor
            .collect(&ctx, data_dir.path())
            .expect("collect succeeds against this crate's manifest");
        let json_path = data_dir.path().join("metadata.json");
        assert!(json_path.exists(), "metadata.json was written");
        let leftovers: Vec<_> = std::fs::read_dir(data_dir.path())
            .expect("read_dir")
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(
            leftovers.is_empty(),
            "atomic_write left a tmp sibling: {leftovers:?}"
        );
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

    #[test]
    fn negative_record_count_surfaces_as_invalid_record_count_error() {
        let raw_count: i64 = -1;
        let result: Result<u64, _> =
            u64::try_from(raw_count).map_err(|_| DbError::InvalidRecordCount {
                table: "metadata_raw".to_string(),
                count: raw_count,
            });
        match result {
            Err(DbError::InvalidRecordCount { table, count }) => {
                assert_eq!(table, "metadata_raw");
                assert_eq!(count, -1);
            }
            _ => panic!("expected InvalidRecordCount error"),
        }
    }
}
