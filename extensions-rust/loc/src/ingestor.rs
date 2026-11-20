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

    fn precheck(&self, ctx: &Context) -> DbResult<()> {
        PIPELINE.check_workspace(ctx.working_directory())
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

    /// A workspace path the sidecar cannot carry is refused by
    /// `precheck`, before `collect` would run the external tool.
    #[test]
    fn precheck_refuses_unrepresentable_workspace_path() {
        let ctx = ops_extension::Context::test_context(std::path::PathBuf::from("/tmp/bad\npath"));
        let err = RustLocIngestor.precheck(&ctx).expect_err("control byte refused");
        match err {
            ops_sqlite::DbError::Io(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
                assert!(e.to_string().contains("cannot be recorded in a sidecar"), "{e}");
            }
            other => panic!("expected DbError::Io, got {other:?}"),
        }
    }
}
