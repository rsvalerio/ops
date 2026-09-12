//! `Sqlite` extension: per-project `SQLite` database for data collection.
//!
//! Tests require `--all-features` or `--features sqlite` to compile.
//! CI must enable the `sqlite` feature flag to run these tests.

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
pub use connection::Sqlite;
pub use error::{DbError, DbResult};
pub use ingestor::{DataIngestor, LoadResult, SidecarIngestorConfig};
// SEC-25 / TASK-2054: `IngestDir` is in `DataIngestor`'s signature, so it must
// be reachable wherever the trait is implemented.
pub use schema::{init_schema, upsert_data_source, DataSourceMetadata, SourceName, WorkspaceRoot};
pub use sql::IngestDir;

use ops_extension::{Context, DataProvider, DataProviderError, ExtensionType};
use std::path::PathBuf;
use std::sync::Arc;

/// SEC-38 / TASK-2018: the `as_ref()` reborrow is load-bearing.
///
/// `ops_extension` gives `SqliteHandle` a blanket impl over every
/// `'static + Send + Sync` type, and `Arc<dyn SqliteHandle>` is itself one of
/// them. Wherever the trait is in scope, method resolution on an `Arc` (or
/// `&Arc`) receiver matches that blanket impl *for the smart pointer* before it
/// derefs, so `as_any()` erases the `Arc` and every downcast to [`Sqlite`]
/// returns `None` — silently, with no error and no compile failure.
///
/// This module happens not to import `SqliteHandle` by name, which is the only
/// reason a bare `h.as_any()` resolves through to the trait object's own method
/// today. That is an accident of the import list, not a contract: a single
/// `use ops_extension::SqliteHandle;` anywhere in this module would flip every
/// downcast to `None`. Reborrowing to `&dyn SqliteHandle` first names the
/// receiver explicitly and drops the dependency on scope entirely. The
/// `sqlite_handle_in_scope` tests below pin both halves.
fn downcast_sqlite(handle: Option<&Arc<dyn ops_extension::SqliteHandle>>) -> Option<&Sqlite> {
    handle.and_then(|h| {
        let erased: &dyn ops_extension::SqliteHandle = h.as_ref();
        erased.as_any().downcast_ref::<Sqlite>()
    })
}

/// Try to provide data from `SQLite` first, falling back to a direct computation.
///
/// Clones the `ctx.db()` Arc to split the borrow so `db_fn` can hold `&Sqlite`
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
    F: FnOnce(&Sqlite, &Context) -> Result<serde_json::Value, anyhow::Error>,
    G: FnOnce(&mut Context) -> Result<serde_json::Value, anyhow::Error>,
{
    let db_arc = ctx.db().cloned();
    if let Some(db) = downcast_sqlite(db_arc.as_ref()) {
        return db_fn(db, ctx).map_err(Into::into);
    }
    fallback_fn(ctx).map_err(Into::into)
}

/// Extract the [`Sqlite`] handle from a context by downcasting from the trait object.
#[must_use]
pub fn get_db(ctx: &Context) -> Option<&Sqlite> {
    downcast_sqlite(ctx.db())
}

// READ-10 / TASK-1873: these are `pub const`s in a library crate, i.e. part of
// the public surface — `dead_code` never fires on them, so the two
// suppressions they used to carry silenced nothing.
/// Extension identifier used to register this crate in the engine's
/// extension registry.
pub const NAME: &str = "sqlite";
/// One-line description shown by `ops about` for this extension.
pub const DESCRIPTION: &str = "Per-project SQLite database for data collection";
/// CLI-facing short name (`db`) used in commands and user-facing output.
pub const SHORTNAME: &str = "db";
/// Registry key of the `sqlite` data provider this crate registers —
/// the key the about code/loc subpages look the database handle up by.
pub const DATA_PROVIDER_NAME: &str = "sqlite";

// TRAIT-9 / TASK-1227: `SqliteHandle` now has a blanket impl over
// `'static + Send + Sync` in `ops_extension::data`, so the explicit
// `impl SqliteHandle for Sqlite` block is no longer needed (and can no
// longer customise the `as_any` body — the canonical `self` body is
// the compile-time-enforced contract).

/// Datasource extension opening the per-project `SQLite` database at the
/// configured path and attaching it to the run's [`Context`].
pub struct SqliteExtension {
    db_path: PathBuf,
}

impl SqliteExtension {
    /// Creates an extension that opens (or creates) the database at
    /// `db_path`.
    #[must_use = "register the returned extension; constructing it opens nothing"]
    pub const fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }
}

ops_extension::impl_extension! {
    SqliteExtension,
    name: NAME,
    description: DESCRIPTION,
    shortname: SHORTNAME,
    types: ExtensionType::DATASOURCE,
    data_provider_name: Some(DATA_PROVIDER_NAME),
    register_data_providers: |this, registry| {
        let _ = registry.register(
            DATA_PROVIDER_NAME,
            Box::new(SqliteProvider {
                db_path: this.db_path.clone(),
            }),
        );
    },
    factory: SQLITE_FACTORY = |config, workspace_root| {
        let db_path = Sqlite::resolve_path(&config.data, workspace_root);
        Some((NAME, Box::new(SqliteExtension::new(db_path))))
    },
}

struct SqliteProvider {
    db_path: PathBuf,
}

impl DataProvider for SqliteProvider {
    fn name(&self) -> &'static str {
        "sqlite"
    }

    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
        if ctx.db().is_some() {
            return Ok(serde_json::Value::Null);
        }
        let db = Sqlite::open(&self.db_path).map_err(DataProviderError::computation_error)?;
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
    fn sqlite_open_in_memory() {
        let db = Sqlite::open_in_memory().expect("should open in-memory db");
        assert_eq!(db.path().to_str(), Some(":memory:"));
    }

    #[test]
    fn sqlite_init_schema_succeeds() {
        let db = Sqlite::open_in_memory().expect("should open");
        init_schema(&db).expect("init_schema should succeed");
    }

    #[test]
    fn sqlite_upsert_and_get_checksum() {
        let db = Sqlite::open_in_memory().expect("should open");
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
    fn sqlite_lock_returns_guard() {
        let db = Sqlite::open_in_memory().expect("should open");
        let guard = db.lock().expect("lock should succeed");
        drop(guard);
    }

    #[test]
    fn sqlite_provider_returns_null() {
        let db = Sqlite::open_in_memory().expect("should open");
        let provider = SqliteProvider {
            db_path: std::path::PathBuf::from(":memory:"),
        };
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));
        ctx.attach_db(std::sync::Arc::new(db));
        let result = provider.provide(&mut ctx).expect("provide should succeed");
        assert!(result.is_null());
    }

    #[test]
    fn sqlite_provider_opens_real_db_when_ctx_db_is_none() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("test_provider.db");
        let provider = SqliteProvider {
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
    fn get_db_returns_some_for_a_context_carrying_a_sqlite_handle() {
        let db = Sqlite::open_in_memory().expect("should open");
        let expected_id = db.id();
        let config = std::sync::Arc::new(ops_core::config::Config::empty());
        let mut ctx = Context::new(config, std::path::PathBuf::from("."));
        ctx.attach_db(std::sync::Arc::new(db));

        let got = get_db(&ctx).expect("handle must downcast back to Sqlite");
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
        let db = Sqlite::open_in_memory().expect("should open");
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
        let db = Sqlite::open_in_memory().expect("should open");
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

    /// SEC-38 / TASK-2018: the accessors above happen to be exercised from a
    /// module where `SqliteHandle` is *not* imported, which is exactly the
    /// condition under which a bare `as_any()` on an `Arc` receiver resolves
    /// correctly. This module imports the trait, so the blanket impl for
    /// `Arc<dyn SqliteHandle>` wins method resolution here — the state the
    /// whole crate is one `use` statement away from. `get_db` and
    /// `try_provide_from_db` must be immune to it.
    mod sqlite_handle_in_scope {
        use super::{get_db, try_provide_from_db, Sqlite};
        use ops_extension::{Context, SqliteHandle};
        use std::sync::Arc;

        fn context_with_a_handle() -> (Context, u64) {
            let db = Sqlite::open_in_memory().expect("should open");
            let id = db.id();
            let config = Arc::new(ops_core::config::Config::empty());
            let mut ctx = Context::new(config, std::path::PathBuf::from("."));
            ctx.attach_db(Arc::new(db));
            (ctx, id)
        }

        #[test]
        fn an_unreborrowed_as_any_on_an_arc_receiver_erases_the_arc() {
            let db = Sqlite::open_in_memory().expect("should open");
            let handle: Arc<dyn SqliteHandle> = Arc::new(db);

            assert!(
                handle.as_any().downcast_ref::<Sqlite>().is_none(),
                "with the trait in scope an Arc receiver does not reach the handle"
            );
            assert!(
                handle
                    .as_any()
                    .downcast_ref::<Arc<dyn SqliteHandle>>()
                    .is_some(),
                "it erases the Arc itself instead"
            );
        }

        #[test]
        fn get_db_finds_the_handle_regardless_of_what_is_in_scope() {
            let (ctx, expected_id) = context_with_a_handle();
            let got = get_db(&ctx).expect("downcast_sqlite must reborrow, not rely on scope");
            assert_eq!(got.id(), expected_id, "must be the very handle attached");
        }

        #[test]
        fn try_provide_from_db_takes_the_db_branch_regardless_of_what_is_in_scope() {
            let (mut ctx, expected_id) = context_with_a_handle();
            let value = try_provide_from_db(
                &mut ctx,
                |db, _ctx| Ok(serde_json::json!({ "id": db.id() })),
                |_ctx| panic!("fallback must not run when a handle is attached"),
            )
            .expect("db branch");
            assert_eq!(value, serde_json::json!({ "id": expected_id }));
        }
    }

    #[test]
    fn sqlite_open_file_based() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("test.db");
        let db = Sqlite::open(&db_path).expect("should open file-based db");
        assert_eq!(db.path(), db_path);
        assert!(db_path.exists());
    }

    #[test]
    fn sqlite_open_creates_parent_directories() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let db_path = temp_dir.path().join("nested/dir/test.db");
        assert!(!db_path.parent().unwrap().exists());
        let _db = Sqlite::open(&db_path).expect("should create parent dirs");
        assert!(db_path.parent().unwrap().exists());
    }
}
