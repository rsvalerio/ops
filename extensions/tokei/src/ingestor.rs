//! TokeiIngestor: collect tokei statistics and load into DuckDB.

use crate::views;
use cargo_ops_duckdb::sql::io_err;
use cargo_ops_duckdb::{DataIngestor, DbResult, DuckDb, LoadResult, SidecarIngestorConfig};
use cargo_ops_extension::Context;
use std::path::Path;

const PIPELINE: SidecarIngestorConfig = SidecarIngestorConfig {
    name: "tokei",
    json_filename: "tokei_files.json",
    count_table: "tokei_files",
    create_label: "tokei_files create",
    view_label: "tokei_languages view",
    count_label: "tokei_files count",
};

pub struct TokeiIngestor;

impl DataIngestor for TokeiIngestor {
    fn name(&self) -> &'static str {
        PIPELINE.name
    }

    fn collect(&self, ctx: &Context, data_dir: &Path) -> DbResult<()> {
        let json = super::collect_tokei(&ctx.working_directory).map_err(io_err)?;
        PIPELINE.collect_sidecar(data_dir, &json, &ctx.working_directory)
    }

    fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<LoadResult> {
        let json_path = data_dir.join(PIPELINE.json_filename);
        let create_sql = views::tokei_files_create_sql(&json_path).map_err(io_err)?;
        let view_sql = views::tokei_languages_view_sql();
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
    fn tokei_ingestor_name() {
        let ingestor = TokeiIngestor;
        assert_eq!(ingestor.name(), "tokei");
    }
}
