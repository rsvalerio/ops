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
pub use schema::{init_schema, upsert_data_source, DataSourceMetadata, SourceName, WorkspaceRoot};

use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};
use std::path::PathBuf;
use std::sync::Arc;

fn downcast_duckdb(handle: &Option<Arc<dyn ops_extension::DuckDbHandle>>) -> Option<&DuckDb> {
    handle
        .as_ref()
        .and_then(|h| h.as_any().downcast_ref::<DuckDb>())
}

/// Try to provide data from DuckDB first, falling back to a direct computation.
///
/// Clones `ctx.db` Arc to split the borrow so `db_fn` can hold `&DuckDb`
/// while `ctx` is still accessible. Arc refcount bump is negligible vs I/O cost.
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
    if let Some(db) = downcast_duckdb(&db_arc) {
        return db_fn(db, ctx).map_err(Into::into);
    }
    fallback_fn(ctx).map_err(Into::into)
}

/// Extract the [`DuckDb`] handle from a context by downcasting from the trait object.
pub fn get_db(ctx: &Context) -> Option<&DuckDb> {
    downcast_duckdb(&ctx.db)
}

pub const NAME: &str = "duckdb";
#[allow(dead_code)]
pub const DESCRIPTION: &str = "Per-project DuckDB database for data collection";
#[allow(dead_code)]
pub const SHORTNAME: &str = "db";
pub const DATA_PROVIDER_NAME: &str = "duckdb";

impl ops_extension::DuckDbHandle for DuckDb {
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

ops_extension::impl_extension! {
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
    factory: DUCKDB_FACTORY = |config, workspace_root| {
        let db_path = DuckDb::resolve_path(&config.data, workspace_root);
        Some((NAME, Box::new(DuckDbExtension::new(db_path))))
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
        if ctx.db.is_some() {
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
    use ops_extension::Context;

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
            &DataSourceMetadata::new(
                SourceName("test_source"),
                WorkspaceRoot("/test/workspace"),
                std::path::Path::new("/test/data.json"),
                42,
                "abc123",
            ),
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
        let config = std::sync::Arc::new(ops_core::config::Config::default());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));
        ctx.db = Some(std::sync::Arc::new(db));
        let result = provider.provide(&mut ctx).expect("provide should succeed");
        assert!(result.is_null());
    }

    #[test]
    fn duck_db_provider_opens_real_db_when_ctx_db_is_none() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("test_provider.duckdb");
        let provider = DuckDbProvider {
            db_path: db_path.clone(),
        };
        let config = std::sync::Arc::new(ops_core::config::Config::default());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));

        assert!(ctx.db.is_none(), "ctx.db should start as None");
        let result = provider.provide(&mut ctx).expect("provide should succeed");
        assert!(result.is_null());
        assert!(ctx.db.is_some(), "ctx.db should be set after provide()");
        assert!(db_path.exists(), "database file should be created");
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
}
