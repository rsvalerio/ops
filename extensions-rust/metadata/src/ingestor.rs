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
            other => external_err(anyhow::Error::new(other).context("cargo metadata")),
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

        // ERR-1 (TASK-1043): the `workspace_root` SELECT below uses
        // `ORDER BY rowid LIMIT 1`, which silently picks an arbitrary row
        // when `metadata_raw` ends up with more than one entry. Today the
        // ingest path produces exactly one row per workspace, but a future
        // schema variant (multi-target, partial re-ingest without truncate)
        // could yield more. Emit a `tracing::warn!` when we observe >1 rows
        // so the discrepancy is observable; the sister read in
        // `query_metadata_raw` already enforces the singleton invariant via
        // `ensure!`. Behaviour is otherwise unchanged: we still take the
        // first row by `rowid`.
        if record_count > 1 {
            tracing::warn!(
                rows = record_count,
                "metadata_raw has multiple workspace_root rows; using first"
            );
        }

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

    /// TASK-0982: regression — path dependencies (source = null) must not be
    /// silently dropped from the `crate_dependencies` view alongside registry
    /// deps.
    #[test]
    fn crate_dependencies_view_includes_path_deps() {
        let data_dir = tempfile::tempdir().unwrap();
        let metadata_json = serde_json::json!({
            "packages": [{
                "name": "test-crate",
                "version": "0.1.0",
                "id": "test-crate 0.1.0 (path+file:///test)",
                "source": "",
                "dependencies": [
                    {
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
                    },
                    {
                        "name": "ws-sibling",
                        "source": null,
                        "req": "*",
                        "kind": "normal",
                        "optional": false,
                        "uses_default_features": true,
                        "features": [],
                        "target": "",
                        "rename": "",
                        "registry": ""
                    }
                ],
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
            "workspace_members": ["test-crate 0.1.0 (path+file:///test)"],
            "workspace_default_members": ["test-crate 0.1.0 (path+file:///test)"],
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
        let _ = ingestor.load(data_dir.path(), &db).unwrap();

        let conn = db.lock().unwrap();
        let total: i64 = conn
            .query_row("SELECT COUNT(*) FROM crate_dependencies", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(total, 2, "both registry and path deps should be present");

        let path_dep_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM crate_dependencies WHERE dependency_name = 'ws-sibling'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(path_dep_count, 1, "path dep (source=null) must be retained");
    }

    /// ERR-1 (TASK-1043): when `metadata_raw` ends up with more than one
    /// row (multi-target metadata, partial re-ingest without truncate), the
    /// `workspace_root` SELECT silently picked an arbitrary first row. The
    /// loader now emits a `tracing::warn!` carrying the row count so the
    /// discrepancy is observable. Drive the path with a JSON array of two
    /// cargo-metadata objects (DuckDB's `read_json_auto` yields one row per
    /// array element) and assert the warn fires.
    #[test]
    fn metadata_load_warns_when_metadata_raw_has_multiple_rows() {
        use ops_about::test_support::TracingBuf;

        fn sample_obj(workspace_root: &str) -> serde_json::Value {
            serde_json::json!({
                "packages": [{
                    "name": "test-crate",
                    "version": "0.1.0",
                    "id": "test-crate 0.1.0 (path+file:///test)",
                    "source": "registry+https://github.com/rust-lang/crates.io-index",
                    "dependencies": [],
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
                "workspace_members": ["test-crate 0.1.0 (path+file:///test)"],
                "workspace_default_members": ["test-crate 0.1.0 (path+file:///test)"],
                "resolve": {"nodes": [], "root": ""},
                "target_directory": "/test/target",
                "version": 1,
                "workspace_root": workspace_root,
                "metadata": {}
            })
        }

        let data_dir = tempfile::tempdir().unwrap();
        // Two-element JSON array → DuckDB `read_json_auto` emits two rows.
        let metadata_json =
            serde_json::Value::Array(vec![sample_obj("/test/a"), sample_obj("/test/b")]);
        let json_path = data_dir.path().join("metadata.json");
        std::fs::write(
            &json_path,
            serde_json::to_vec_pretty(&metadata_json).unwrap(),
        )
        .unwrap();

        let buf = TracingBuf::default();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf.clone())
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();

        let db = DuckDb::open_in_memory().expect("open in-memory db");
        let ingestor = MetadataIngestor;
        let result =
            tracing::subscriber::with_default(subscriber, || ingestor.load(data_dir.path(), &db));
        assert!(result.is_ok(), "load should succeed (warn-only path)");

        let logs = buf.captured();
        assert!(
            logs.contains("multiple workspace_root rows"),
            "expected warn about multiple workspace_root rows, got: {logs}"
        );
        assert!(
            logs.contains("rows=2"),
            "warn should include rows=2 field, got: {logs}"
        );
    }

    /// PATTERN-1 / TASK-1056: the same dependency declared under two
    /// `[target.'cfg(...)'.dependencies]` blocks must surface as TWO
    /// distinct rows in `crate_dependencies` (preserving the
    /// platform-specific shape via the new `target` column) rather than
    /// collapsing into a single tuple. cargo metadata serialises each
    /// declaration as its own entry in `package.dependencies`, so the
    /// view must keep both — TASK-0982 fixed the inverse drop, this
    /// fixes the duplicate-collapse.
    #[test]
    fn crate_dependencies_view_preserves_target_conditional_duplicates() {
        let data_dir = tempfile::tempdir().unwrap();
        let metadata_json = serde_json::json!({
            "packages": [{
                "name": "test-crate",
                "version": "0.1.0",
                "id": "test-crate 0.1.0 (path+file:///test)",
                "source": "registry+https://github.com/rust-lang/crates.io-index",
                "dependencies": [
                    {
                        "name": "libc",
                        "source": "registry+https://github.com/rust-lang/crates.io-index",
                        "req": "^0.2",
                        "kind": "normal",
                        "optional": false,
                        "uses_default_features": true,
                        "features": [],
                        "target": "cfg(unix)",
                        "rename": "",
                        "registry": ""
                    },
                    {
                        "name": "libc",
                        "source": "registry+https://github.com/rust-lang/crates.io-index",
                        "req": "^0.2",
                        "kind": "normal",
                        "optional": false,
                        "uses_default_features": true,
                        "features": [],
                        "target": "cfg(windows)",
                        "rename": "",
                        "registry": ""
                    }
                ],
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
            "workspace_members": ["test-crate 0.1.0 (path+file:///test)"],
            "workspace_default_members": ["test-crate 0.1.0 (path+file:///test)"],
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
        let _ = ingestor.load(data_dir.path(), &db).unwrap();

        let conn = db.lock().unwrap();
        let total: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM crate_dependencies WHERE dependency_name = 'libc'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            total, 2,
            "both target-conditional libc declarations must surface as distinct rows"
        );

        // The new `target` column must carry the cfg expression so
        // platform-specific shape isn't lost.
        let mut targets: Vec<String> = Vec::new();
        let mut stmt = conn
            .prepare(
                "SELECT target FROM crate_dependencies \
                 WHERE dependency_name = 'libc' \
                 ORDER BY target",
            )
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok(row
                    .get::<_, Option<String>>(0)?
                    .unwrap_or_else(|| "<null>".to_string()))
            })
            .unwrap();
        for r in rows {
            targets.push(r.unwrap());
        }
        assert_eq!(targets, vec!["cfg(unix)", "cfg(windows)"]);
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
