//! DataIngestor trait for loading data into DuckDb.

use crate::connection::DuckDb;
use crate::error::DbResult;
use ops_extension::Context;
use std::path::Path;

/// Result of a load operation (record count, etc.).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct LoadResult {
    pub source_name: &'static str,
    pub record_count: u64,
}

#[allow(dead_code)]
impl LoadResult {
    pub fn success(source_name: &'static str, record_count: u64) -> Self {
        Self {
            source_name,
            record_count,
        }
    }
}

/// Configuration for a sidecar-based ingestor pipeline (DUP-001).
///
/// Captures the static parameters shared by ingestors that use workspace sidecar
/// files (e.g., tokei, coverage). The methods handle the common collect/load/checksum
/// workflow, eliminating duplicated boilerplate across ingestor implementations.
#[allow(dead_code)]
pub struct SidecarIngestorConfig {
    pub name: &'static str,
    pub json_filename: &'static str,
    pub count_table: &'static str,
    pub create_label: &'static str,
    pub view_label: &'static str,
    pub count_label: &'static str,
}

#[allow(dead_code)]
impl SidecarIngestorConfig {
    /// Write serializable data to JSON and create workspace sidecar.
    pub fn collect_sidecar(
        &self,
        data_dir: &Path,
        data: &impl serde::Serialize,
        working_directory: &Path,
    ) -> DbResult<()> {
        std::fs::create_dir_all(data_dir).map_err(crate::error::DbError::Io)?;
        let json_bytes = serde_json::to_vec_pretty(data).map_err(crate::sql::io_err)?;
        let json_path = data_dir.join(self.json_filename);
        std::fs::write(&json_path, &json_bytes).map_err(crate::error::DbError::Io)?;
        crate::sql::write_workspace_sidecar(data_dir, self.name, working_directory)?;
        Ok(())
    }

    /// Standard load pipeline: init schema → create table → create view → count →
    /// read sidecar → upsert → cleanup.
    pub fn load_with_sidecar(
        &self,
        db: &DuckDb,
        data_dir: &Path,
        create_sql: &str,
        view_sql: &str,
    ) -> DbResult<crate::ingestor::LoadResult> {
        crate::schema::init_schema(db)?;
        let conn = db.lock()?;

        conn.execute(create_sql, [])
            .map_err(|e| crate::error::DbError::query_failed(self.create_label, e))?;
        conn.execute(view_sql, [])
            .map_err(|e| crate::error::DbError::query_failed(self.view_label, e))?;

        let record_count: u64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM {}", self.count_table),
                [],
                |row: &duckdb::Row| row.get::<_, i64>(0),
            )
            .map_err(|e| crate::error::DbError::query_failed(self.count_label, e))?
            as u64;

        let workspace_root = crate::sql::read_workspace_sidecar(data_dir, self.name)?;
        drop(conn);

        let json_path = data_dir.join(self.json_filename);
        let checksum = crate::sql::checksum_file(&json_path)?;
        crate::schema::upsert_data_source(
            db,
            self.name,
            &workspace_root,
            &json_path,
            record_count,
            &checksum,
        )?;

        std::fs::remove_file(&json_path).map_err(crate::error::DbError::Io)?;
        crate::sql::remove_workspace_sidecar(data_dir, self.name);

        Ok(LoadResult::success(self.name, record_count))
    }

    /// Compute checksum of the JSON file.
    pub fn checksum(&self, data_dir: &Path) -> DbResult<String> {
        crate::sql::checksum_file(&data_dir.join(self.json_filename))
    }
}

/// Trait for data sources that collect raw data and load it into DuckDB.
///
/// Implementations handle the full lifecycle of external data:
/// 1. **Collect**: Run external commands or read files to produce JSON
/// 2. **Load**: Parse JSON and load into DuckDB tables/views
/// 3. **Checksum**: Compute hash for skip-if-unchanged optimization
///
/// # Lifecycle
///
/// The `refresh_metadata` function orchestrates the typical flow:
/// 1. Call `checksum()` to compare with stored checksum
/// 2. If changed, call `collect()` to gather fresh data
/// 3. Call `load()` to ingest into DuckDB
///
/// # Example
///
/// ```ignore
/// struct MetadataIngestor;
///
/// impl DataIngestor for MetadataIngestor {
///     fn name(&self) -> &'static str { "metadata" }
///     fn collect(&self, ctx: &Context, data_dir: &Path) -> DbResult<()> {
///         // Run `cargo metadata` and write to data_dir
///     }
///     fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<LoadResult> {
///         // Read JSON and create DuckDB view
///     }
///     fn checksum(&self, data_dir: &Path) -> DbResult<String> {
///         // SHA-256 of the JSON file
///     }
/// }
/// ```
#[allow(dead_code)]
pub trait DataIngestor: Send + Sync {
    /// Unique source name (e.g. "metadata", "tokei").
    ///
    /// Used as the primary key in the `data_sources` tracking table.
    fn name(&self) -> &'static str;

    /// Collect raw data (run external commands, produce JSON files).
    ///
    /// This method runs the external tool (e.g., `cargo metadata`) and
    /// writes the output to files in `data_dir`. It should not interact
    /// with the database.
    fn collect(&self, ctx: &Context, data_dir: &Path) -> DbResult<()>;

    /// Load collected data into DuckDB tables/views.
    ///
    /// This method reads files from `data_dir` and creates or replaces
    /// tables/views in the database. Should be idempotent.
    fn load(&self, data_dir: &Path, db: &DuckDb) -> DbResult<LoadResult>;

    /// Compute checksum for skip-if-unchanged logic.
    ///
    /// Returns a hash (typically SHA-256) of the source data. If this
    /// matches the stored checksum, `load()` may be skipped.
    fn checksum(&self, data_dir: &Path) -> DbResult<String>;
}
