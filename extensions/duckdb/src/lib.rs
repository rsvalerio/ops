//! DuckDb extension: per-project DuckDB database for data collection.
//!
//! Tests require `--all-features` or `--features duckdb` to compile.
//! CI must enable the `duckdb` feature flag to run these tests.

mod connection;
mod error;
mod ingestor;
mod schema;
pub mod sql;

#[allow(unused_imports)]
pub use connection::DuckDb;
#[allow(unused_imports)]
pub use error::{DbError, DbResult};
#[allow(unused_imports)]
pub use ingestor::{DataIngestor, LoadResult, SidecarIngestorConfig};
#[allow(unused_imports)]
pub use schema::{init_schema, upsert_data_source};

use cargo_ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};
use std::path::PathBuf;
use std::sync::Arc;

/// DUP-008: Try to provide data from DuckDB first, falling back to a direct computation.
///
/// Clones the `ctx.db` Arc to split the borrow — the DuckDB reference is used by `db_fn`
/// while `ctx` is passed mutably to `fallback_fn`.
pub fn try_provide_from_db<F, G>(
    ctx: &mut Context,
    db_fn: F,
    fallback_fn: G,
) -> Result<serde_json::Value, DataProviderError>
where
    F: FnOnce(&DuckDb, &Context) -> Result<serde_json::Value, anyhow::Error>,
    G: FnOnce(&mut Context) -> Result<serde_json::Value, anyhow::Error>,
{
    let db_arc = ctx.db.clone();
    let db_ref = db_arc
        .as_ref()
        .and_then(|h| h.as_any().downcast_ref::<DuckDb>());
    if let Some(db) = db_ref {
        return db_fn(db, ctx).map_err(Into::into);
    }
    fallback_fn(ctx).map_err(Into::into)
}

pub const NAME: &str = "duckdb";
#[allow(dead_code)]
pub const DESCRIPTION: &str = "Per-project DuckDB database for data collection";
#[allow(dead_code)]
pub const SHORTNAME: &str = "db";
pub const DATA_PROVIDER_NAME: &str = "duckdb";

impl cargo_ops_extension::DuckDbHandle for DuckDb {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct DuckDbExtension {
    db_path: PathBuf,
}

impl DuckDbExtension {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }
}

cargo_ops_extension::impl_extension! {
    DuckDbExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |this, registry| {
        registry.register(
            DATA_PROVIDER_NAME,
            Box::new(DuckDbProvider {
                db_path: this.db_path.clone(),
            }),
        );
    },
}

struct DuckDbProvider {
    db_path: PathBuf,
}

impl DataProvider for DuckDbProvider {
    fn name(&self) -> &'static str {
        "duckdb"
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        if let Some(ref db) = ctx.db {
            let _ = db;
            return Ok(serde_json::Value::Null);
        }
        let db = DuckDb::open(&self.db_path).map_err(DataProviderError::computation_error)?;
        init_schema(&db).map_err(DataProviderError::computation_error)?;
        ctx.db = Some(Arc::new(db));
        Ok(serde_json::Value::Null)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingestor::{DataIngestor, LoadResult};
    use cargo_ops_extension::Context;
    use std::io::Write;

    #[test]
    fn duck_db_open_in_memory() {
        let db = DuckDb::open_in_memory().expect("should open in-memory db");
        assert_eq!(db.path().to_str(), Some(":memory:"));
    }

    #[test]
    fn duck_db_init_schema_succeeds() {
        let db = DuckDb::open_in_memory().expect("should open");
        init_schema(&db).expect("init_schema should succeed");
    }

    #[test]
    fn duck_db_upsert_and_get_checksum() {
        let db = DuckDb::open_in_memory().expect("should open");
        init_schema(&db).expect("init_schema");
        upsert_data_source(
            &db,
            "test_source",
            "/test/workspace",
            std::path::Path::new("/test/data.json"),
            42,
            "abc123",
        )
        .expect("upsert should succeed");
        let checksum = schema::get_source_checksum(&db, "test_source", "/test/workspace")
            .expect("get should succeed");
        assert_eq!(checksum, Some("abc123".to_string()));
    }

    #[test]
    fn duck_db_lock_returns_guard() {
        let db = DuckDb::open_in_memory().expect("should open");
        let guard = db.lock().expect("lock should succeed");
        drop(guard);
    }

    #[test]
    fn duck_db_provider_returns_null() {
        let db = DuckDb::open_in_memory().expect("should open");
        let provider = DuckDbProvider {
            db_path: std::path::PathBuf::from(":memory:"),
        };
        let config = std::sync::Arc::new(cargo_ops_core::config::Config::default());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));
        ctx.db = Some(std::sync::Arc::new(db));
        let result = provider.provide(&mut ctx).expect("provide should succeed");
        assert!(result.is_null());
    }

    /// TQ-012: Test the real DB open path (lines 64-67 of DuckDbProvider::provide).
    /// This tests opening a new database when ctx.db is None.
    #[test]
    fn duck_db_provider_opens_real_db_when_ctx_db_is_none() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("test_provider.duckdb");
        let provider = DuckDbProvider {
            db_path: db_path.clone(),
        };
        let config = std::sync::Arc::new(cargo_ops_core::config::Config::default());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));

        assert!(ctx.db.is_none(), "ctx.db should start as None");

        let result = provider.provide(&mut ctx).expect("provide should succeed");
        assert!(result.is_null());

        assert!(ctx.db.is_some(), "ctx.db should be set after provide()");
        assert!(db_path.exists(), "database file should be created");
    }

    #[test]
    fn db_error_mutex_poisoned_message() {
        let err = DbError::MutexPoisoned("test panic".to_string());
        let msg = err.to_string();
        assert!(msg.contains("test panic"));
    }

    #[test]
    fn db_error_query_failed_context() {
        let err = DbError::query_failed(
            "test_op",
            duckdb::Error::InvalidParameterName("test".into()),
        );
        let msg = err.to_string();
        assert!(msg.contains("test_op"));
    }

    #[test]
    fn duck_db_open_file_based() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("test.duckdb");
        let db = DuckDb::open(&db_path).expect("should open file-based db");
        assert_eq!(db.path(), db_path);
        assert!(db_path.exists());
    }

    #[test]
    fn duck_db_open_creates_parent_directories() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("nested/dir/test.duckdb");
        assert!(!db_path.parent().unwrap().exists());
        let _db = DuckDb::open(&db_path).expect("should create parent dirs");
        assert!(db_path.parent().unwrap().exists());
    }

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

        fn collect(&self, _ctx: &Context, data_dir: &std::path::Path) -> DbResult<()> {
            let json_path = data_dir.join("data.json");
            let mut file = std::fs::File::create(&json_path).map_err(DbError::Io)?;
            write!(file, r#"{{"test": "data"}}"#).map_err(DbError::Io)?;
            Ok(())
        }

        fn load(&self, data_dir: &std::path::Path, _db: &DuckDb) -> DbResult<LoadResult> {
            let json_path = data_dir.join("data.json");
            if json_path.exists() {
                Ok(LoadResult::success(self.name, 1))
            } else {
                Ok(LoadResult::success(self.name, 0))
            }
        }

        fn checksum(&self, data_dir: &std::path::Path) -> DbResult<String> {
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
        let config = std::sync::Arc::new(cargo_ops_core::config::Config::default());
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

        let checksum_before = ingestor
            .checksum(temp_dir.path())
            .expect("checksum should succeed");
        assert_eq!(checksum_before, "empty");

        std::fs::write(temp_dir.path().join("data.json"), r#"{"test": "data"}"#).unwrap();
        let checksum_after = ingestor
            .checksum(temp_dir.path())
            .expect("checksum should succeed");
        assert_eq!(checksum_after, "mock_checksum");
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

    /// TQ-014: Error path tests for DataIngestor trait implementations.
    mod ingestor_error_tests {
        use super::*;

        struct FailingCollectIngestor;

        impl DataIngestor for FailingCollectIngestor {
            fn name(&self) -> &'static str {
                "failing_collect"
            }

            fn collect(&self, _ctx: &Context, _data_dir: &std::path::Path) -> DbResult<()> {
                Err(DbError::Io(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "collect failed",
                )))
            }

            fn load(&self, _data_dir: &std::path::Path, _db: &DuckDb) -> DbResult<LoadResult> {
                Ok(LoadResult::success(self.name(), 0))
            }

            fn checksum(&self, _data_dir: &std::path::Path) -> DbResult<String> {
                Ok("test".to_string())
            }
        }

        #[test]
        fn ingestor_collect_error_propagates() {
            let ingestor = FailingCollectIngestor;
            let config = std::sync::Arc::new(cargo_ops_core::config::Config::default());
            let ctx = Context::new(config, std::path::PathBuf::from("."));
            let temp_dir = tempfile::tempdir().expect("tempdir");

            let result = ingestor.collect(&ctx, temp_dir.path());
            assert!(result.is_err(), "collect error should propagate");
            let err = result.unwrap_err();
            assert!(err.to_string().contains("collect failed"));
        }

        struct FailingChecksumIngestor;

        impl DataIngestor for FailingChecksumIngestor {
            fn name(&self) -> &'static str {
                "failing_checksum"
            }

            fn collect(&self, _ctx: &Context, data_dir: &std::path::Path) -> DbResult<()> {
                std::fs::create_dir_all(data_dir).map_err(DbError::Io)?;
                Ok(())
            }

            fn load(&self, _data_dir: &std::path::Path, _db: &DuckDb) -> DbResult<LoadResult> {
                Ok(LoadResult::success(self.name(), 0))
            }

            fn checksum(&self, data_dir: &std::path::Path) -> DbResult<String> {
                let path = data_dir.join("nonexistent.json");
                std::fs::read(&path).map_err(DbError::Io)?;
                Ok("unreachable".to_string())
            }
        }

        #[test]
        fn ingestor_checksum_missing_file_error() {
            let ingestor = FailingChecksumIngestor;
            let temp_dir = tempfile::tempdir().expect("tempdir");

            let result = ingestor.checksum(temp_dir.path());
            assert!(result.is_err(), "checksum should fail for missing file");
        }
    }
}
