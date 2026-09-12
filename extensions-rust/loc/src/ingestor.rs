//! `RustLocIngestor`: collect Rust LOC statistics and load into `SQLite`.

use crate::views;
use ops_extension::Context;
use ops_sqlite::sql::external_err;
use ops_sqlite::{DataIngestor, DbResult, IngestDir, LoadResult, SidecarIngestorConfig, Sqlite};

const PIPELINE: SidecarIngestorConfig =
    SidecarIngestorConfig::new("rust-loc", "rust_loc_files.json", "rust_loc_files");

/// Sidecar ingestor persisting Rust LOC statistics as `rust_loc_files.json`
/// and loading them into the `rust_loc_files` table plus `rust_loc_summary`
/// view.
pub struct RustLocIngestor;

impl DataIngestor for RustLocIngestor {
    fn name(&self) -> &'static str {
        PIPELINE.name
    }

    fn collect(&self, ctx: &Context, dir: &IngestDir) -> DbResult<()> {
        let json = super::collect_rust_loc(ctx.working_directory(), ctx.deadline_handle().as_ref())
            .map_err(external_err)?;
        PIPELINE.collect_sidecar(dir, &json, ctx.working_directory())
    }

    fn load(&self, dir: &IngestDir, db: &Sqlite) -> DbResult<LoadResult> {
        let view_sql = views::rust_loc_summary_view_sql();
        PIPELINE.load_with_sidecar(db, dir, &views::RUST_LOC_FILES_LOAD, &view_sql)
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
