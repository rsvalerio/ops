//! `RustLocIngestor`: collect Rust LOC statistics and load into `DuckDB`.

use crate::views;
use ops_duckdb::sql::external_err;
use ops_duckdb::{DataIngestor, DbResult, DuckDb, LoadResult, SidecarIngestorConfig};
use ops_extension::Context;
use std::path::Path;

const PIPELINE: SidecarIngestorConfig =
    SidecarIngestorConfig::new("rust-loc", "rust_loc_files.json", "rust_loc_files");

pub struct RustLocIngestor;

impl DataIngestor for RustLocIngestor {
    fn name(&self) -> &'static str {
        PIPELINE.name
    }

    fn collect(&self, ctx: &Context, data_dir: &Path) -> DbResult<()> {
        let json = super::collect_rust_loc(&ctx.working_directory).map_err(external_err)?;
        PIPELINE.collect_sidecar(data_dir, &json, &ctx.working_directory)
    }

    fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<LoadResult> {
        let json_path = data_dir.join(PIPELINE.json_filename);
        let create_sql = views::rust_loc_files_create_sql(&json_path)?;
        let view_sql = views::rust_loc_summary_view_sql();
        PIPELINE.load_with_sidecar(db, data_dir, &create_sql, &view_sql)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_loc_ingestor_name() {
        assert_eq!(RustLocIngestor.name(), "rust-loc");
    }
}
