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

    #[test]
    fn coverage_ingestor_name() {
        let ingestor = CoverageIngestor;
        assert_eq!(ingestor.name(), "coverage");
    }
}
