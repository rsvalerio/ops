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
    /// Interpolated into a `SELECT COUNT(*) FROM "{count_table}"` query.
    /// Must be a valid SQL identifier — the `&'static str` bound keeps this
    /// compile-time, and `validate_identifier` enforces it at runtime in
    /// `load_with_sidecar` as defense-in-depth if the type is ever widened.
    pub count_table: &'static str,
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
            .map_err(|e| crate::error::DbError::query_failed(format!("{} create", self.name), e))?;
        conn.execute(view_sql, [])
            .map_err(|e| crate::error::DbError::query_failed(format!("{} view", self.name), e))?;

        crate::sql::validate_identifier(self.count_table).unwrap_or_else(|e| {
            panic!(
                "SidecarIngestorConfig.count_table {:?} is not a valid SQL identifier: {}",
                self.count_table, e
            )
        });
        let record_count: u64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM \"{}\"", self.count_table),
                [],
                |row: &duckdb::Row| row.get::<_, i64>(0),
            )
            .map_err(|e| {
                crate::error::DbError::query_failed(format!("{} count", self.count_table), e)
            })
            .map(|v| u64::try_from(v).unwrap_or(0))?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{connection::DuckDb, error::DbError};
    use std::io::Write;

    #[test]
    fn load_result_success() {
        let result = LoadResult::success("test_source", 100);
        assert_eq!(result.source_name, "test_source");
        assert_eq!(result.record_count, 100);
    }

    struct MockIngestor {
        name: &'static str,
    }

    impl DataIngestor for MockIngestor {
        fn name(&self) -> &'static str {
            self.name
        }

        fn collect(&self, _ctx: &Context, data_dir: &Path) -> DbResult<()> {
            let json_path = data_dir.join("data.json");
            let mut file = std::fs::File::create(&json_path).map_err(DbError::Io)?;
            write!(file, r#"{{"test": "data"}}"#).map_err(DbError::Io)?;
            Ok(())
        }

        fn load(&self, data_dir: &Path, _db: &DuckDb) -> DbResult<LoadResult> {
            let json_path = data_dir.join("data.json");
            if json_path.exists() {
                Ok(LoadResult::success(self.name, 1))
            } else {
                Ok(LoadResult::success(self.name, 0))
            }
        }

        fn checksum(&self, data_dir: &Path) -> DbResult<String> {
            let json_path = data_dir.join("data.json");
            if json_path.exists() {
                Ok("mock_checksum".to_string())
            } else {
                Ok("empty".to_string())
            }
        }
    }

    #[test]
    fn data_ingestor_trait_collect() {
        let ingestor = MockIngestor { name: "test" };
        let config = std::sync::Arc::new(ops_core::config::Config::default());
        let ctx = Context::new(config, std::path::PathBuf::from("."));
        let temp_dir = tempfile::tempdir().expect("tempdir");
        ingestor
            .collect(&ctx, temp_dir.path())
            .expect("collect should succeed");
        assert!(temp_dir.path().join("data.json").exists());
    }

    #[test]
    fn data_ingestor_trait_checksum() {
        let ingestor = MockIngestor { name: "test" };
        let temp_dir = tempfile::tempdir().expect("tempdir");

        assert_eq!(ingestor.checksum(temp_dir.path()).unwrap(), "empty");
        std::fs::write(temp_dir.path().join("data.json"), r#"{"test": "data"}"#).unwrap();
        assert_eq!(ingestor.checksum(temp_dir.path()).unwrap(), "mock_checksum");
    }

    #[test]
    fn data_ingestor_trait_load() {
        let ingestor = MockIngestor { name: "test" };
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db = DuckDb::open_in_memory().expect("db");
        std::fs::write(temp_dir.path().join("data.json"), r#"{"test": "data"}"#).unwrap();
        let result = ingestor
            .load(temp_dir.path(), &db)
            .expect("load should succeed");
        assert_eq!(result.source_name, "test");
        assert_eq!(result.record_count, 1);
    }

    mod ingestor_error_tests {
        use super::*;

        struct FailingCollectIngestor;

        impl DataIngestor for FailingCollectIngestor {
            fn name(&self) -> &'static str {
                "failing_collect"
            }
            fn collect(&self, _ctx: &Context, _data_dir: &Path) -> DbResult<()> {
                Err(DbError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "collect failed",
                )))
            }
            fn load(&self, _data_dir: &Path, _db: &DuckDb) -> DbResult<LoadResult> {
                Ok(LoadResult::success(self.name(), 0))
            }
            fn checksum(&self, _data_dir: &Path) -> DbResult<String> {
                Ok("test".to_string())
            }
        }

        #[test]
        fn ingestor_collect_error_propagates() {
            let ingestor = FailingCollectIngestor;
            let config = std::sync::Arc::new(ops_core::config::Config::default());
            let ctx = Context::new(config, std::path::PathBuf::from("."));
            let temp_dir = tempfile::tempdir().expect("tempdir");
            let result = ingestor.collect(&ctx, temp_dir.path());
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("collect failed"));
        }

        struct FailingChecksumIngestor;

        impl DataIngestor for FailingChecksumIngestor {
            fn name(&self) -> &'static str {
                "failing_checksum"
            }
            fn collect(&self, _ctx: &Context, data_dir: &Path) -> DbResult<()> {
                std::fs::create_dir_all(data_dir).map_err(DbError::Io)?;
                Ok(())
            }
            fn load(&self, _data_dir: &Path, _db: &DuckDb) -> DbResult<LoadResult> {
                Ok(LoadResult::success(self.name(), 0))
            }
            fn checksum(&self, data_dir: &Path) -> DbResult<String> {
                let path = data_dir.join("nonexistent.json");
                std::fs::read(&path).map_err(DbError::Io)?;
                Ok("unreachable".to_string())
            }
        }

        #[test]
        fn ingestor_checksum_missing_file_error() {
            let ingestor = FailingChecksumIngestor;
            let temp_dir = tempfile::tempdir().expect("tempdir");
            assert!(ingestor.checksum(temp_dir.path()).is_err());
        }
    }
}
