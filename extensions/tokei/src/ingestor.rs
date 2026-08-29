//! `TokeiIngestor`: collect tokei statistics and load into `DuckDB`.

use crate::views;
use ops_duckdb::sql::external_err;
use ops_duckdb::{DataIngestor, DbResult, DuckDb, IngestDir, LoadResult, SidecarIngestorConfig};
use ops_extension::Context;

const PIPELINE: SidecarIngestorConfig =
    SidecarIngestorConfig::new("tokei", "tokei_files.json", "tokei_files");

pub struct TokeiIngestor;

impl DataIngestor for TokeiIngestor {
    fn name(&self) -> &'static str {
        PIPELINE.name
    }

    fn collect(&self, ctx: &Context, dir: &IngestDir) -> DbResult<()> {
        let json = super::collect_tokei(ctx.working_directory(), ctx.deadline_handle().as_ref())
            .map_err(external_err)?;
        PIPELINE.collect_sidecar(dir, &json, ctx.working_directory())
    }

    fn load(&self, dir: &IngestDir, db: &DuckDb) -> DbResult<LoadResult> {
        let json_path = dir.entry_path(PIPELINE.json_filename);
        let create_sql = views::tokei_files_create_sql(&json_path)?;
        let view_sql = views::tokei_languages_view_sql();
        PIPELINE.load_with_sidecar(db, dir, &create_sql, &view_sql)
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
