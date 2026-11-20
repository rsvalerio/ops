//! `TokeiIngestor`: collect tokei statistics and load into `SQLite`.

use crate::views;
use ops_extension::Context;
use ops_sqlite::sql::external_err;
use ops_sqlite::{DataIngestor, DbResult, IngestDir, LoadResult, SidecarIngestorConfig, Sqlite};

const PIPELINE: SidecarIngestorConfig =
    SidecarIngestorConfig::new("tokei", "tokei_files.json", "tokei_files");

/// Sidecar ingestor persisting tokei statistics as `tokei_files.json` and
/// loading them into the `tokei_files` table plus `tokei_languages` view.
pub struct TokeiIngestor;

impl DataIngestor for TokeiIngestor {
    fn name(&self) -> &'static str {
        PIPELINE.name
    }

    fn precheck(&self, ctx: &Context) -> DbResult<()> {
        PIPELINE.check_workspace(ctx.working_directory())
    }

    fn collect(&self, ctx: &Context, dir: &IngestDir) -> DbResult<()> {
        let json = super::collect_tokei(ctx.working_directory(), ctx.deadline_handle().as_ref())
            .map_err(external_err)?;
        PIPELINE.collect_sidecar(dir, &json, ctx.working_directory())
    }

    fn load(&self, dir: &IngestDir, db: &Sqlite) -> DbResult<LoadResult> {
        let view_sql = views::tokei_languages_view_sql();
        PIPELINE.load_with_sidecar(db, dir, &views::TOKEI_FILES_LOAD, &view_sql)
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
        let err = TokeiIngestor.precheck(&ctx).expect_err("control byte refused");
        match err {
            ops_sqlite::DbError::Io(e) => {
                assert_eq!(e.kind(), std::io::ErrorKind::InvalidInput);
                assert!(e.to_string().contains("cannot be recorded in a sidecar"), "{e}");
            }
            other => panic!("expected DbError::Io, got {other:?}"),
        }
    }

    #[test]
    fn tokei_ingestor_name() {
        let ingestor = TokeiIngestor;
        assert_eq!(ingestor.name(), "tokei");
    }
}
