//! MetadataIngestor: collect cargo metadata and load into DuckDB.

use crate::views;
use crate::{check_metadata_output, run_cargo_metadata};
use ops_duckdb::sql::io_err;
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
        let output = run_cargo_metadata(&ctx.working_directory).map_err(DbError::Io)?;
        check_metadata_output(&output).map_err(io_err)?;
        let path = data_dir.join("metadata.json");
        std::fs::write(&path, &output.stdout).map_err(DbError::Io)?;
        Ok(())
    }

    fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<LoadResult> {
        init_schema(db)?;
        let conn = db.lock()?;

        let path = data_dir.join("metadata.json");
        let sql = views::metadata_raw_create_sql(&path).map_err(io_err)?;
        conn.execute(&sql, [])
            .map_err(|e| DbError::query_failed("metadata_raw create", e))?;

        let view_sql = views::crate_dependencies_view_sql();
        conn.execute(&view_sql, [])
            .map_err(|e| DbError::query_failed("crate_dependencies view", e))?;

        let workspace_root: String = conn
            .query_row(
                "SELECT workspace_root FROM metadata_raw LIMIT 1",
                [],
                |row| row.get(0),
            )
            .map_err(|e| DbError::query_failed("metadata_raw workspace_root extract", e))?;

        drop(conn);

        let record_count = 1u64;
        let checksum = ops_duckdb::sql::checksum_file(&data_dir.join("metadata.json"))?;
        upsert_data_source(
            db,
            self.name(),
            &workspace_root,
            &path,
            record_count,
            &checksum,
        )?;

        // Note: File is deleted after successful load. If the load fails before this point,
        // the staged file remains and can be re-loaded. This is intentional - it allows
        // recovery from transient failures without re-running cargo metadata.
        std::fs::remove_file(&path).map_err(DbError::Io)?;

        Ok(LoadResult::success(self.name(), record_count))
    }

    fn checksum(&self, data_dir: &Path) -> DbResult<String> {
        ops_duckdb::sql::checksum_file(&data_dir.join("metadata.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metadata_ingestor_name() {
        let ingestor = MetadataIngestor;
        assert_eq!(ingestor.name(), "metadata");
    }
}
