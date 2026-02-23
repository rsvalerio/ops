//! OpsDb extension: per-project DuckDB database for data collection.

mod connection;
mod error;
mod ingestor;
mod schema;

#[allow(unused_imports)]
pub use connection::OpsDb;
#[allow(unused_imports)]
pub use error::{DbError, DbResult};
#[allow(unused_imports)]
pub use ingestor::{DataIngestor, LoadResult};
pub use schema::{init_schema, upsert_data_source};

use crate::extension::CommandRegistry;
use crate::extension::{
    Context, DataProvider, DataProviderError, DataRegistry, Extension, ExtensionType, OpsDbHandle,
};
use std::path::PathBuf;
use std::sync::Arc;

pub const NAME: &str = "ops-db";
#[allow(dead_code)]
pub const DESCRIPTION: &str = "Per-project DuckDB database for data collection";
#[allow(dead_code)]
pub const SHORTNAME: &str = "db";
pub const DATA_PROVIDER_NAME: &str = "ops_db";

pub struct OpsDbExtension {
    db_path: PathBuf,
}

impl OpsDbExtension {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }
}

impl Extension for OpsDbExtension {
    fn name(&self) -> &'static str {
        NAME
    }

    fn description(&self) -> &'static str {
        DESCRIPTION
    }

    fn shortname(&self) -> &'static str {
        SHORTNAME
    }

    fn types(&self) -> ExtensionType {
        ExtensionType::DATASOURCE
    }

    fn data_provider_name(&self) -> Option<&'static str> {
        Some(DATA_PROVIDER_NAME)
    }

    fn register_commands(&self, _registry: &mut CommandRegistry) {}

    fn register_data_providers(&self, registry: &mut DataRegistry) {
        registry.register(
            DATA_PROVIDER_NAME,
            Box::new(OpsDbProvider {
                db_path: self.db_path.clone(),
            }),
        );
    }
}

struct OpsDbProvider {
    db_path: PathBuf,
}

impl DataProvider for OpsDbProvider {
    fn name(&self) -> &'static str {
        "ops_db"
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        if let Some(ref db) = ctx.db {
            let _ = db;
            return Ok(serde_json::Value::Null);
        }
        let db = OpsDb::open(&self.db_path)
            .map_err(|e| DataProviderError::computation_failed(e.to_string()))?;
        init_schema(&db).map_err(|e| DataProviderError::computation_failed(e.to_string()))?;
        ctx.db = Some(Arc::new(db));
        Ok(serde_json::Value::Null)
    }
}

impl OpsDbHandle for OpsDb {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::extension::Context;
    use crate::extensions::ops_db::ingestor::{DataIngestor, LoadResult};
    use std::io::Write;

    #[test]
    fn ops_db_open_in_memory() {
        let db = OpsDb::open_in_memory().expect("should open in-memory db");
        assert_eq!(db.path().to_str(), Some(":memory:"));
    }

    #[test]
    fn ops_db_init_schema_succeeds() {
        let db = OpsDb::open_in_memory().expect("should open");
        init_schema(&db).expect("init_schema should succeed");
    }

    #[test]
    fn ops_db_upsert_and_get_checksum() {
        let db = OpsDb::open_in_memory().expect("should open");
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
    fn ops_db_lock_returns_guard() {
        let db = OpsDb::open_in_memory().expect("should open");
        let guard = db.lock().expect("lock should succeed");
        drop(guard);
    }

    #[test]
    fn ops_db_provider_returns_null() {
        let db = OpsDb::open_in_memory().expect("should open");
        let provider = OpsDbProvider {
            db_path: std::path::PathBuf::from(":memory:"),
        };
        let config = std::sync::Arc::new(crate::config::Config::default());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));
        ctx.db = Some(std::sync::Arc::new(db));
        let result = provider.provide(&mut ctx).expect("provide should succeed");
        assert!(result.is_null());
    }

    /// TQ-012: Test the real DB open path (lines 64-67 of OpsDbProvider::provide).
    /// This tests opening a new database when ctx.db is None.
    #[test]
    fn ops_db_provider_opens_real_db_when_ctx_db_is_none() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("test_provider.duckdb");
        let provider = OpsDbProvider {
            db_path: db_path.clone(),
        };
        let config = std::sync::Arc::new(crate::config::Config::default());
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
    fn ops_db_open_file_based() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("test.duckdb");
        let db = OpsDb::open(&db_path).expect("should open file-based db");
        assert_eq!(db.path(), db_path);
        assert!(db_path.exists());
    }

    #[test]
    fn ops_db_open_creates_parent_directories() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("nested/dir/test.duckdb");
        assert!(!db_path.parent().unwrap().exists());
        let _db = OpsDb::open(&db_path).expect("should create parent dirs");
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

        fn load(&self, data_dir: &std::path::Path, _db: &OpsDb) -> DbResult<LoadResult> {
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
        let config = std::sync::Arc::new(crate::config::Config::default());
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
        let db = OpsDb::open_in_memory().expect("db");

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

            fn load(&self, _data_dir: &std::path::Path, _db: &OpsDb) -> DbResult<LoadResult> {
                Ok(LoadResult::success(self.name(), 0))
            }

            fn checksum(&self, _data_dir: &std::path::Path) -> DbResult<String> {
                Ok("test".to_string())
            }
        }

        #[test]
        fn ingestor_collect_error_propagates() {
            let ingestor = FailingCollectIngestor;
            let config = std::sync::Arc::new(crate::config::Config::default());
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

            fn load(&self, _data_dir: &std::path::Path, _db: &OpsDb) -> DbResult<LoadResult> {
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
