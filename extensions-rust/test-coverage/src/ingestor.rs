//! CoverageIngestor: collect LLVM coverage data and load into DuckDB.

use crate::views;
use ops_duckdb::sql::io_err;
use ops_duckdb::{DataIngestor, DbResult, DuckDb, LoadResult, SidecarIngestorConfig};
use ops_extension::Context;
use std::path::Path;

const PIPELINE: SidecarIngestorConfig = SidecarIngestorConfig {
    name: "coverage",
    json_filename: "coverage_files.json",
    count_table: "coverage_files",
    create_label: "coverage_files create",
    view_label: "coverage_summary view",
    count_label: "coverage_files count",
};

pub struct CoverageIngestor;

impl DataIngestor for CoverageIngestor {
    fn name(&self) -> &'static str {
        PIPELINE.name
    }

    fn collect(&self, ctx: &Context, data_dir: &Path) -> DbResult<()> {
        let records = super::collect_coverage(&ctx.working_directory).map_err(io_err)?;
        PIPELINE.collect_sidecar(data_dir, &records, &ctx.working_directory)
    }

    fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<LoadResult> {
        let json_path = data_dir.join(PIPELINE.json_filename);
        let create_sql = views::coverage_files_create_sql(&json_path).map_err(io_err)?;
        let view_sql = views::coverage_summary_view_sql();
        PIPELINE.load_with_sidecar(db, data_dir, &create_sql, &view_sql)
    }

    fn checksum(&self, data_dir: &Path) -> DbResult<String> {
        PIPELINE.checksum(data_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ops_duckdb::DuckDb;
    use std::path::PathBuf;

    #[test]
    fn coverage_ingestor_name() {
        let ingestor = CoverageIngestor;
        assert_eq!(ingestor.name(), "coverage");
    }

    #[test]
    fn coverage_collect_fails_with_nonexistent_directory() {
        let ingestor = CoverageIngestor;
        let ctx =
            ops_extension::Context::test_context(PathBuf::from("/nonexistent/path/to/project"));
        let data_dir = tempfile::tempdir().unwrap();
        let result = ingestor.collect(&ctx, data_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn coverage_checksum_fails_when_file_missing() {
        let data_dir = tempfile::tempdir().unwrap();
        let ingestor = CoverageIngestor;
        let result = ingestor.checksum(data_dir.path());
        assert!(result.is_err());
    }

    #[test]
    fn coverage_load_with_sample_data() {
        let data_dir = tempfile::tempdir().unwrap();
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
        let json_path = data_dir.path().join(PIPELINE.json_filename);
        std::fs::write(
            &json_path,
            serde_json::to_vec_pretty(&coverage_json).unwrap(),
        )
        .unwrap();

        // Write workspace sidecar
        ops_duckdb::sql::write_workspace_sidecar(
            data_dir.path(),
            PIPELINE.name,
            working_dir.path(),
        )
        .unwrap();

        let db = DuckDb::open_in_memory().expect("open in-memory db");
        let ingestor = CoverageIngestor;
        let result = ingestor.load(data_dir.path(), &db);
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
        assert_eq!(lines_count, 100);

        // Verify JSON file was cleaned up
        assert!(!json_path.exists());
    }
}
