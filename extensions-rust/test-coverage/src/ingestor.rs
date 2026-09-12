//! `CoverageIngestor`: collect LLVM coverage data and load into `SQLite`.

use crate::parse::collect_coverage;
use crate::views;
use ops_extension::Context;
use ops_sqlite::sql::external_err;
use ops_sqlite::{DataIngestor, DbResult, IngestDir, LoadResult, SidecarIngestorConfig, Sqlite};

const PIPELINE: SidecarIngestorConfig =
    SidecarIngestorConfig::new("coverage", "coverage_files.json", "coverage_files");

pub struct CoverageIngestor;

impl DataIngestor for CoverageIngestor {
    fn name(&self) -> &'static str {
        PIPELINE.name
    }

    fn collect(&self, ctx: &Context, dir: &IngestDir) -> DbResult<()> {
        let records =
            collect_coverage(ctx.working_directory(), ctx.deadline()).map_err(external_err)?;
        PIPELINE.collect_sidecar(dir, &records, ctx.working_directory())
    }

    fn load(&self, dir: &IngestDir, db: &Sqlite) -> DbResult<LoadResult> {
        let view_sql = views::coverage_summary_view_sql();
        PIPELINE.load_with_sidecar(db, dir, &views::COVERAGE_FILES_LOAD, &view_sql)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_sqlite::Sqlite;

    #[test]
    fn coverage_ingestor_name() {
        let ingestor = CoverageIngestor;
        assert_eq!(ingestor.name(), "coverage");
    }

    #[test]
    fn coverage_collect_fails_with_nonexistent_directory() {
        let ingestor = CoverageIngestor;
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("does-not-exist");
        let ctx = ops_extension::Context::test_context(missing);
        let data_dir = tempfile::tempdir().unwrap();
        // SEC-25 / TASK-2054: stage through the same verified anchor
        // `provide_via_ingestor` builds.
        let dir = IngestDir::open(&data_dir.path().join("ingest")).expect("anchor");
        let result = ingestor.collect(&ctx, &dir);
        assert!(result.is_err());
    }

    /// DUP-3 / TASK-1562: this test deliberately keeps its own single-file
    /// fixture (rather than going through `crate::tests::setup_loaded_db`)
    /// because it owns the `WHERE filename = 'src/lib.rs'` round-trip
    /// assertion against `lines_count = 100`. The shared fixture in
    /// `tests::sample_coverage_json` deliberately ships two files (and
    /// `src/lib.rs` carries `lines_count = 200`) so the
    /// `coverage_summary_view_*` tests can pin the SUM-across-files
    /// aggregates. Using the shared fixture would force a value rewrite
    /// here that hides the original assertion's intent.
    #[test]
    fn coverage_load_with_sample_data() {
        let data_dir = tempfile::tempdir().unwrap();
        // SEC-25 / TASK-2054: stage through the same verified anchor
        // `provide_via_ingestor` builds.
        let dir = IngestDir::open(&data_dir.path().join("ingest")).expect("anchor");
        let working_dir = tempfile::tempdir().unwrap();

        // Write sample coverage JSON
        let coverage_json = serde_json::json!([
            {
                "filename": "src/lib.rs",
                "lines_count": 100,
                "lines_covered": 80,
                "lines_percent": 80.0,
                "functions_count": 10,
                "functions_covered": 8,
                "functions_percent": 80.0,
                "regions_count": 20,
                "regions_covered": 16,
                "regions_notcovered": 4,
                "regions_percent": 80.0,
                "branches_count": 5,
                "branches_covered": 4,
                "branches_notcovered": 1,
                "branches_percent": 80.0
            }
        ]);
        dir.write_atomic(
            PIPELINE.json_filename,
            &serde_json::to_vec_pretty(&coverage_json).unwrap(),
        )
        .unwrap();

        // Write workspace sidecar
        ops_sqlite::sql::write_workspace_sidecar(&dir, PIPELINE.name, working_dir.path()).unwrap();

        let db = Sqlite::open_in_memory().expect("open in-memory db");
        let ingestor = CoverageIngestor;
        let result = ingestor.load(&dir, &db);
        assert!(result.is_ok());
        let load_result = result.unwrap();
        assert_eq!(load_result.source_name, "coverage");
        assert_eq!(load_result.record_count, 1);

        // Verify data is queryable
        let conn = db.lock().unwrap();
        let lines_count: i64 = conn
            .query_row(
                "SELECT lines_count FROM coverage_files WHERE filename = 'src/lib.rs'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        drop(conn);
        assert_eq!(lines_count, 100);

        // Verify JSON file was cleaned up
        assert!(!dir.entry_path(PIPELINE.json_filename).exists());
    }
}
