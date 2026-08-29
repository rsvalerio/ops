//! `DuckDb` extension: per-project `DuckDB` database for data collection.
//!
//! Tests require `--all-features` or `--features duckdb` to compile.
//! CI must enable the `duckdb` feature flag to run these tests.

#![cfg_attr(
    test,
    allow(
        clippy::unwrap_used,
        clippy::cast_possible_truncation,
        clippy::cast_precision_loss,
        clippy::cast_sign_loss
    )
)]

mod connection;
mod error;
mod ingestor;
mod schema;
pub mod sql;

// READ-10 / TASK-1873: no `#[allow(unused_imports)]` here. A `pub use` in a
// library crate is a re-export and is never "unused", so the four
// suppressions this block used to carry silenced nothing.
pub use connection::DuckDb;
pub use error::{DbError, DbResult};
pub use ingestor::{DataIngestor, LoadResult, SidecarIngestorConfig};
pub use schema::{init_schema, upsert_data_source, DataSourceMetadata, SourceName, WorkspaceRoot};

use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};
use std::path::PathBuf;
use std::sync::Arc;

fn downcast_duckdb(handle: Option<&Arc<dyn ops_extension::DuckDbHandle>>) -> Option<&DuckDb> {
    handle.and_then(|h| h.as_any().downcast_ref::<DuckDb>())
}

/// Try to provide data from `DuckDB` first, falling back to a direct computation.
///
/// Clones the `ctx.db()` Arc to split the borrow so `db_fn` can hold `&DuckDb`
/// while `ctx` is still accessible. Arc refcount bump is negligible vs I/O cost.
///
/// # Errors
///
/// [`DataProviderError`] if `db_fn` fails when a database is available, or
/// if `fallback_fn` fails when it is not.
pub fn try_provide_from_db<F, G>(
    ctx: &mut Context,
    db_fn: F,
    fallback_fn: G,
) -> Result<serde_json::Value, DataProviderError>
where
    F: FnOnce(&DuckDb, &Context) -> Result<serde_json::Value, anyhow::Error>,
    G: FnOnce(&mut Context) -> Result<serde_json::Value, anyhow::Error>,
{
    let db_arc = ctx.db().cloned();
    if let Some(db) = downcast_duckdb(db_arc.as_ref()) {
        return db_fn(db, ctx).map_err(Into::into);
    }
    fallback_fn(ctx).map_err(Into::into)
}

/// Extract the [`DuckDb`] handle from a context by downcasting from the trait object.
#[must_use]
pub fn get_db(ctx: &Context) -> Option<&DuckDb> {
    downcast_duckdb(ctx.db())
}

// READ-10 / TASK-1873: these are `pub const`s in a library crate, i.e. part of
// the public surface — `dead_code` never fires on them, so the two
// suppressions they used to carry silenced nothing.
pub const NAME: &str = "duckdb";
pub const DESCRIPTION: &str = "Per-project DuckDB database for data collection";
pub const SHORTNAME: &str = "db";
pub const DATA_PROVIDER_NAME: &str = "duckdb";

// TRAIT-9 / TASK-1227: `DuckDbHandle` now has a blanket impl over
// `'static + Send + Sync` in `ops_extension::data`, so the explicit
// `impl DuckDbHandle for DuckDb` block is no longer needed (and can no
// longer customise the `as_any` body — the canonical `self` body is
// the compile-time-enforced contract).

pub struct DuckDbExtension {
    db_path: PathBuf,
}

impl DuckDbExtension {
    #[must_use]
    pub const fn new(db_path: PathBuf) -> Self {
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
        let _ = registry.register(
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
        if ctx.db().is_some() {
            return Ok(serde_json::Value::Null);
        }
        let db = DuckDb::open(&self.db_path).map_err(DataProviderError::computation_error)?;
        init_schema(&db).map_err(DataProviderError::computation_error)?;
        ctx.attach_db(Arc::new(db));
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
                SourceName::new("test_source"),
                WorkspaceRoot::new(std::ffi::OsStr::new("/test/workspace")),
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
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));
        ctx.attach_db(std::sync::Arc::new(db));
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
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));

        assert!(ctx.db().is_none(), "ctx.db() should start as None");
        let result = provider.provide(&mut ctx).expect("provide should succeed");
        assert!(result.is_null());
        assert!(ctx.db().is_some(), "ctx.db() should be set after provide()");
        assert!(db_path.exists(), "database file should be created");
    }

    // --- try_provide_from_db / get_db (TEST-5 / TASK-1870) ---
    //
    // Both sit on the boundary between this crate and every consumer of it,
    // and both fail *softly* — a regression shows up as a slower path or a
    // blank about-page section, never as a red test. Pin the branch
    // contracts.

    #[test]
    fn get_db_returns_some_for_a_context_carrying_a_duckdb_handle() {
        let db = DuckDb::open_in_memory().expect("should open");
        let expected_id = db.id();
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));
        ctx.attach_db(std::sync::Arc::new(db));

        let got = get_db(&ctx).expect("handle must downcast back to DuckDb");
        assert_eq!(got.id(), expected_id, "must be the very handle attached");
    }

    #[test]
    fn get_db_returns_none_without_a_handle() {
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let ctx = Context::new(config, std::path::PathBuf::from("."));
        assert!(get_db(&ctx).is_none());
    }

    #[test]
    fn try_provide_from_db_takes_the_db_branch_when_a_handle_is_attached() {
        let db = DuckDb::open_in_memory().expect("should open");
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));
        ctx.attach_db(std::sync::Arc::new(db));

        let value = try_provide_from_db(
            &mut ctx,
            |_db, _ctx| Ok(serde_json::json!({"from": "db"})),
            |_ctx| panic!("fallback must not run when a handle is attached"),
        )
        .expect("db branch");
        assert_eq!(value, serde_json::json!({"from": "db"}));
    }

    #[test]
    fn try_provide_from_db_takes_the_fallback_branch_without_a_handle() {
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));

        let value = try_provide_from_db(
            &mut ctx,
            |_db, _ctx| panic!("db branch must not run without a handle"),
            |_ctx| Ok(serde_json::json!({"from": "fallback"})),
        )
        .expect("fallback branch");
        assert_eq!(value, serde_json::json!({"from": "fallback"}));
    }

    #[test]
    fn try_provide_from_db_maps_the_db_branch_error_into_data_provider_error() {
        let db = DuckDb::open_in_memory().expect("should open");
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));
        ctx.attach_db(std::sync::Arc::new(db));

        let err = try_provide_from_db(
            &mut ctx,
            |_db, _ctx| Err(anyhow::anyhow!("db branch exploded")),
            |_ctx| Ok(serde_json::Value::Null),
        )
        .expect_err("db-branch failure must propagate, not fall back");
        assert!(
            err.to_string().contains("db branch exploded"),
            "cause must survive the conversion: {err}"
        );
    }

    #[test]
    fn try_provide_from_db_maps_the_fallback_error_into_data_provider_error() {
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));

        let err = try_provide_from_db(
            &mut ctx,
            |_db, _ctx| Ok(serde_json::Value::Null),
            |_ctx| Err(anyhow::anyhow!("fallback exploded")),
        )
        .expect_err("fallback failure must propagate");
        assert!(
            err.to_string().contains("fallback exploded"),
            "cause must survive the conversion: {err}"
        );
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
