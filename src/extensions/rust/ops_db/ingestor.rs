//! DataIngestor trait for loading data into OpsDb.

use crate::extension::Context;
use crate::extensions::ops_db::connection::OpsDb;
use crate::extensions::ops_db::error::DbResult;
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
///     fn load(&self, data_dir: &Path, db: &OpsDb) -> DbResult<LoadResult> {
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
    fn load(&self, data_dir: &Path, db: &OpsDb) -> DbResult<LoadResult>;

    /// Compute checksum for skip-if-unchanged logic.
    ///
    /// Returns a hash (typically SHA-256) of the source data. If this
    /// matches the stored checksum, `load()` may be skipped.
    fn checksum(&self, data_dir: &Path) -> DbResult<String>;
}
