//! `MetadataIngestor`: collect cargo metadata and load into `DuckDB`.

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
        let path = data_dir.join("metadata.json");
        {
            let conn = db.lock()?;
            build_views(&conn, &path)?;
        }
        let conn = db.lock()?;
        let record_count = query_record_count(&conn)?;
        if record_count > 1 {
            tracing::warn!(
                rows = record_count,
                "metadata_raw has multiple workspace_root rows; using first"
            );
        }
        let workspace_root = extract_workspace_root(&conn)?;
        drop(conn);

        let checksum = ops_duckdb::sql::checksum_file(&path)?;
        upsert_data_source(
            db,
            &ops_duckdb::DataSourceMetadata::new(
                ops_duckdb::SourceName::new(self.name()),
                ops_duckdb::WorkspaceRoot::new(std::ffi::OsStr::new(&workspace_root)),
                &path,
                record_count,
                &checksum,
            ),
        )?;
        cleanup_staged_file(&path);
        Ok(LoadResult::success(self.name(), record_count))
    }
}

/// FN-1 / TASK-1543: build the `metadata_raw` table and `crate_dependencies`
/// view in one place. Extracted from `MetadataIngestor::load` so the loader
/// reads at one nesting level.
fn build_views(conn: &duckdb::Connection, path: &Path) -> DbResult<()> {
    let sql = views::metadata_raw_create_sql(path)?;
    conn.execute(&sql, [])
        .map_err(|e| DbError::query_failed("metadata_raw create", e))?;
    let view_sql = views::crate_dependencies_view_sql();
    conn.execute(&view_sql, [])
        .map_err(|e| DbError::query_failed("crate_dependencies view", e))?;
    Ok(())
}

/// FN-1 / TASK-1543: count rows in `metadata_raw` and map the raw `i64` to
/// `u64` via the project's `InvalidRecordCount` policy (API-1 / TASK-0606).
fn query_record_count(conn: &duckdb::Connection) -> DbResult<u64> {
    let raw: i64 = conn
        .query_row("SELECT count(*) FROM metadata_raw", [], |row| {
            row.get::<_, i64>(0)
        })
        .map_err(|e| DbError::query_failed("metadata_raw count", e))?;
    u64::try_from(raw).map_err(|_| DbError::InvalidRecordCount {
        table: "metadata_raw".to_string(),
        count: raw,
    })
}

/// FN-1 / TASK-1543: read the first `workspace_root` from `metadata_raw`,
/// enriching the error with the column type probe pinned by READ-5 /
/// TASK-0614.
fn extract_workspace_root(conn: &duckdb::Connection) -> DbResult<String> {
    conn.query_row(
        "SELECT workspace_root FROM metadata_raw ORDER BY rowid LIMIT 1",
        [],
        |row| row.get(0),
    )
    .map_err(|e| {
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
    })
}

/// FN-1 / TASK-1543: best-effort removal of the staged JSON file after a
/// successful load. TASK-0510: a failure here must not propagate — the
/// `DuckDB` row is already committed and a subsequent re-ingest would
/// otherwise loop.
fn cleanup_staged_file(path: &Path) {
    if let Err(e) = std::fs::remove_file(path) {
        tracing::warn!(
            path = %path.display(),
            error = %e,
            "failed to remove staged metadata file after successful load; leaving in place"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{ingest_dep, ingest_metadata, write_metadata_json};

    #[test]
    fn metadata_ingestor_name() {
        let ingestor = MetadataIngestor;
        assert_eq!(ingestor.name(), "metadata");
    }

    /// TEST-1 / TASK-1546: pin the failure mode to "cargo ran but couldn't
    /// locate a Cargo.toml" rather than asserting only `is_err()`. The bare
    /// assertion passed for the wrong reason on environments without
    /// `cargo` on `PATH` (`DbError::Io`) and on slow CI hits
    /// (`DbError::Timeout`), neither of which is what the test name
    /// promises. Match on `DbError::External` whose Display chain mentions
    /// `cargo metadata` so the test fails loudly if the upstream failure
    /// path stops surfacing the cargo origin.
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
        let err = ingestor
            .collect(&ctx, data_dir.path())
            .expect_err("collect must fail on a missing working directory");
        match &err {
            DbError::External(inner) => {
                let chain = format!("{inner:#}");
                assert!(
                    chain.contains("cargo metadata"),
                    "External error should attribute to cargo metadata; got: {chain}"
                );
            }
            DbError::Io(_) => panic!(
                "expected a cargo-metadata External error, got DbError::Io \
                 (is `cargo` on PATH?): {err}"
            ),
            other => panic!("expected DbError::External, got: {other:?}"),
        }
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

        let metadata_json = ingest_metadata().dep(ingest_dep("serde", "^1.0")).value();
        let json_path = write_metadata_json(data_dir.path(), &metadata_json);

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
        let metadata_json = ingest_metadata()
            .source(serde_json::json!(""))
            .dep(ingest_dep("serde", "^1.0"))
            .dep(ingest_dep("ws-sibling", "*").path_source())
            .value();
        write_metadata_json(data_dir.path(), &metadata_json);

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
    /// cargo-metadata objects (`DuckDB`'s `read_json_auto` yields one row per
    /// array element) and assert the warn fires.
    #[test]
    fn metadata_load_warns_when_metadata_raw_has_multiple_rows() {
        use ops_about::test_support::TracingBuf;

        let data_dir = tempfile::tempdir().unwrap();
        // Two-element JSON array → DuckDB `read_json_auto` emits two rows.
        let metadata_json = serde_json::Value::Array(vec![
            ingest_metadata().root("/test/a").value(),
            ingest_metadata().root("/test/b").value(),
        ]);
        write_metadata_json(data_dir.path(), &metadata_json);

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
        let metadata_json = ingest_metadata()
            .dep(ingest_dep("libc", "^0.2").target("cfg(unix)"))
            .dep(ingest_dep("libc", "^0.2").target("cfg(windows)"))
            .value();
        write_metadata_json(data_dir.path(), &metadata_json);

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

    /// FN-1 / TASK-1543 AC#2: drive the `extract_workspace_root` typeof-probe
    /// fallback by handing it a `metadata_raw` shape whose `workspace_root`
    /// column is `INTEGER`-typed (the JSON ingest path coerces null-only
    /// columns to INTEGER). The probe should observe the type and surface
    /// it in the error so the operator sees the offending shape.
    #[test]
    fn extract_workspace_root_typeof_probe_surfaces_observed_type() {
        let db = DuckDb::open_in_memory().expect("open in-memory db");
        let conn = db.lock().expect("acquire connection");
        conn.execute("CREATE TABLE metadata_raw (workspace_root INTEGER)", [])
            .expect("create table");
        conn.execute("INSERT INTO metadata_raw VALUES (42)", [])
            .expect("seed row");
        let err = super::extract_workspace_root(&conn)
            .expect_err("INTEGER workspace_root cannot deserialise to String");
        let rendered = format!("{err:#}");
        assert!(
            rendered.contains("observed type: INTEGER"),
            "typeof-probe must name observed column type; got: {rendered}"
        );
    }

    // TEST-1 / TASK-1546: the previous `negative_record_count_surfaces_as_…`
    // test constructed `u64::try_from(-1)` inline and pattern-matched the
    // error it created itself — it exercised no production code path. The
    // `InvalidRecordCount` mapping in `MetadataIngestor::load` (see lines
    // ~67-72 above) is already exercised by the loader's existing
    // success-path tests and by the broader DuckDB record-count plumbing
    // in `ops-duckdb`; a dedicated tautology test added no coverage and
    // gave reviewers false confidence, so it has been removed.
}
