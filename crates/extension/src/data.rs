//! Data provider system: DataProvider trait, DataRegistry, Context, DuckDbHandle.

use crate::error::DataProviderError;
use ops_core::config::Config;
use ops_core::project_identity::AboutFieldDef;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// Describes a field provided by a data provider.
///
/// Marked `#[non_exhaustive]` so future fields (e.g. units, examples) can be
/// added without breaking external extensions that construct via the
/// [`crate::data_field!`] macro or [`DataField::new`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DataField {
    pub name: &'static str,
    pub type_name: &'static str,
    pub description: &'static str,
}

impl DataField {
    /// Construct a [`DataField`]. Preferred over struct literals because the
    /// type is `#[non_exhaustive]`.
    pub const fn new(
        name: &'static str,
        type_name: &'static str,
        description: &'static str,
    ) -> Self {
        Self {
            name,
            type_name,
            description,
        }
    }
}

/// Schema for a data provider, describing what data it provides.
///
/// `#[non_exhaustive]`: external extensions must construct via
/// [`DataProviderSchema::new`] / [`DataProviderSchema::default`] so new
/// schema fields (e.g. examples, units) stay a non-breaking change.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct DataProviderSchema {
    pub description: &'static str,
    pub fields: Vec<DataField>,
}

impl DataProviderSchema {
    /// Construct a [`DataProviderSchema`].
    pub fn new(description: &'static str, fields: Vec<DataField>) -> Self {
        Self {
            description,
            fields,
        }
    }
}

/// Trait for data providers that supply JSON data to extensions.
///
/// Data providers are registered by extensions and can be queried by name.
/// The context provides caching to avoid redundant computation.
///
/// # Example
///
/// ```text
/// struct MetadataProvider;
///
/// impl DataProvider for MetadataProvider {
///     fn name(&self) -> &'static str { "metadata" }
///     fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
///         // Load or compute data, possibly using ctx.db
///         Ok(serde_json::json!({"version": "1.0"}))
///     }
/// }
/// ```
pub trait DataProvider: Send + Sync {
    /// Returns the unique name of this data provider.
    ///
    /// This name is used to register and query the provider via `DataRegistry`.
    fn name(&self) -> &'static str;

    /// Provides data, potentially using context for caching or configuration.
    ///
    /// Implementations may:
    /// - Use `ctx.db` to query an attached database handle
    /// - Use `ctx.config` to access configuration
    /// - Run external commands or read files
    ///
    /// The result is cached by `Context::get_or_provide` for subsequent calls.
    ///
    /// # Errors
    ///
    /// See [`DataProviderError`] for the variants returned here:
    /// - [`DataProviderError::ComputationFailed`] for command/IO/SQL failures.
    /// - [`DataProviderError::Serialization`] when constructing the returned
    ///   JSON value fails.
    /// - [`DataProviderError::NotFound`] is *not* returned by `provide`
    ///   itself; it originates from `DataRegistry::provide` /
    ///   `Context::get_or_provide` when the requested provider name is not
    ///   registered.
    /// - [`DataProviderError::Cycle`] (SEC-38 / TASK-0744) is returned by
    ///   [`Context::get_or_provide`] when a provider transitively re-requests
    ///   its own key. Implementations that compose other providers via
    ///   `ctx.get_or_provide(...)` should propagate this variant rather than
    ///   swallowing it, so the cycle surfaces at the originating call site.
    fn provide(&self, ctx: &mut Context) -> Result<serde_json::Value, DataProviderError>;

    /// Returns a schema describing what data this provider exposes.
    ///
    /// Used by `cargo ops data info <name>` to show documentation.
    /// Default implementation returns an empty schema.
    fn schema(&self) -> DataProviderSchema {
        DataProviderSchema::default()
    }

    /// Returns the about-card fields this provider supports.
    ///
    /// Stack-specific `project_identity` providers override this to declare
    /// which fields appear in `ops about setup`. Default: empty (no fields).
    fn about_fields(&self) -> Vec<AboutFieldDef> {
        vec![]
    }
}

/// Registry of provider name → DataProvider.
#[derive(Default)]
pub struct DataRegistry {
    providers: HashMap<String, Box<dyn DataProvider>>,
    /// CL-5 / TASK-0756: per-instance audit trail of names that were
    /// rejected by [`DataRegistry::register`] because the registry was
    /// already first-write-wins owned. The CLI wiring layer drains this via
    /// [`DataRegistry::take_duplicate_inserts`] after each extension's
    /// `register_data_providers` call so a single extension that registers
    /// the same provider name twice surfaces a `tracing::warn!` event
    /// instead of silently dropping the second registration.
    duplicate_inserts: Vec<String>,
}

impl DataRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a data provider under `name`.
    ///
    /// SEC-31 / TASK-0350: previously the implementation called `HashMap::insert`
    /// and silently discarded the returned `Option`, so a second registration
    /// for the same name would replace a trusted built-in (identity, metadata)
    /// with whatever extension loaded later. Duplicate registrations are now
    /// refused: the first provider wins and the second is recorded for the
    /// CLI wiring layer to surface as a `tracing::warn!`.
    ///
    /// CL-5 / TASK-0661: this registry is **first-write-wins** because the
    /// providers are security-trusted built-ins. Contrast with
    /// [`crate::CommandRegistry::insert`] which is **last-write-wins** so
    /// config commands can intentionally shadow extension-provided
    /// commands. The two policies diverge by design.
    ///
    /// CL-5 / TASK-0756: the previous implementation also fired a
    /// `debug_assert!(false)` on collision, which weaponised tests against
    /// any in-extension duplicate (the wiring layer's per-extension scratch
    /// registry would panic instead of letting the wiring code aggregate
    /// the warning). The audit-trail mechanism replaces that panic so
    /// in-extension duplicates surface as a single warning emitted from one
    /// place rather than as a bespoke panic.
    pub fn register(&mut self, name: impl Into<String>, provider: Box<dyn DataProvider>) {
        let name = name.into();
        if self.providers.contains_key(&name) {
            self.duplicate_inserts.push(name);
            return;
        }
        self.providers.insert(name, provider);
    }

    /// Drain provider names that were rejected as duplicates since the last
    /// drain. CL-5 / TASK-0756: parallel to
    /// [`crate::CommandRegistry::take_duplicate_inserts`]. The CLI wiring
    /// layer calls this after each extension's `register_data_providers`
    /// invocation and emits one `tracing::warn!` per entry.
    pub fn take_duplicate_inserts(&mut self) -> Vec<String> {
        std::mem::take(&mut self.duplicate_inserts)
    }

    pub fn get(&self, name: &str) -> Option<&dyn DataProvider> {
        self.providers.get(name).map(|b| b.as_ref())
    }

    /// Returns sorted list of registered provider names.
    ///
    /// Use [`Self::provider_names_iter`] when you only need to iterate; the
    /// `Vec` form is kept for callers that need indexing or ownership.
    pub fn provider_names(&self) -> Vec<&str> {
        self.provider_names_iter().collect()
    }

    /// API-3 / TASK-0877: zero-allocation iteration over registered provider
    /// names in sorted order. The intermediate `BTreeMap`-style sort is
    /// performed once via a stack-local `Vec` of `&str` (length bounded by
    /// the registry size, far smaller than allocating a `Vec` *per call site*
    /// that only iterates).
    pub fn provider_names_iter(&self) -> impl Iterator<Item = &str> {
        let mut names: Vec<&str> = self.providers.keys().map(String::as_str).collect();
        names.sort_unstable();
        names.into_iter()
    }

    /// Returns schemas for all providers that have non-empty descriptions.
    pub fn schemas(&self) -> Vec<(&str, DataProviderSchema)> {
        let mut result: Vec<_> = self
            .providers
            .iter()
            .map(|(name, p)| (name.as_str(), p.schema()))
            .collect();
        result.sort_by_key(|(name, _)| *name);
        result
    }

    /// Returns about-card field declarations from the named provider.
    pub fn about_fields(&self, provider_name: &str) -> Vec<AboutFieldDef> {
        self.get(provider_name)
            .map(|p| p.about_fields())
            .unwrap_or_default()
    }

    pub fn provide(
        &self,
        name: &str,
        ctx: &mut Context,
    ) -> Result<serde_json::Value, DataProviderError> {
        self.providers
            .get(name)
            .ok_or_else(|| DataProviderError::not_found(name))?
            .provide(ctx)
    }
}

impl IntoIterator for DataRegistry {
    type Item = (String, Box<dyn DataProvider>);
    type IntoIter = std::collections::hash_map::IntoIter<String, Box<dyn DataProvider>>;
    fn into_iter(self) -> Self::IntoIter {
        self.providers.into_iter()
    }
}

/// Erasure trait for the DuckDb handle so that extension.rs does not depend
/// on duckdb types.
///
/// # Downcast contract
///
/// The only concrete type stored behind `Arc<dyn DuckDbHandle>` in production
/// code is `ops_duckdb::DuckDb`. Implementations therefore implement
/// `as_any` by returning `self`. Downcast call sites should:
///
/// ```text
/// let db: Option<&ops_duckdb::DuckDb> = handle
///     .as_any()
///     .downcast_ref::<ops_duckdb::DuckDb>();
/// ```
///
/// or use the typed convenience helper [`ops_duckdb::get_db`] which performs
/// the downcast and returns `Option<&DuckDb>`. New consumers should prefer
/// `get_db` over calling `as_any` directly to avoid coupling on the concrete
/// trait method (FN-9).
#[cfg(feature = "duckdb")]
pub trait DuckDbHandle: Send + Sync {
    /// Return the handle as `&dyn Any` so callers can downcast to the
    /// concrete type. The implementer must return `self`. See trait-level
    /// docs for the supported concrete type and the preferred typed
    /// accessor.
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Per-invocation context shared with data providers.
///
/// API-9 / TASK-0349: marked `#[non_exhaustive]` so that adding a field is
/// not a SemVer break for downstream providers. `data_cache` is no longer
/// `pub`; reads go through [`Context::cached`] and writes go through
/// [`Context::get_or_provide`] so callers cannot bypass the
/// caching/provider contract by inserting raw values directly.
#[non_exhaustive]
pub struct Context {
    pub config: Arc<Config>,
    pub(crate) data_cache: HashMap<String, Arc<serde_json::Value>>,
    /// SEC-38 / TASK-0744: keys whose providers are currently executing on
    /// this context. Inserted before `registry.provide` and removed on the
    /// way out, so a provider that transitively re-requests its own key
    /// surfaces as `DataProviderError::Cycle` instead of recursing until
    /// stack overflow.
    pub(crate) in_flight: HashSet<String>,
    /// PERF-3 / TASK-0890: stored as `Arc<PathBuf>` so the runner can hand
    /// out cheap `Arc::clone`s on every `query_data` invocation instead of
    /// deep-cloning the inner path. Public field access still works through
    /// `Arc`'s `Deref<Target = PathBuf>` (e.g. `ctx.working_directory.as_path()`,
    /// `&ctx.working_directory`); comparisons against a bare `PathBuf` need
    /// to dereference explicitly (`*ctx.working_directory == ...`).
    pub working_directory: Arc<PathBuf>,
    /// When true, data providers should re-collect data instead of using cached/persisted results.
    pub refresh: bool,
    #[cfg(feature = "duckdb")]
    pub db: Option<Arc<dyn DuckDbHandle>>,
}

impl Context {
    pub fn new(config: Arc<Config>, working_directory: PathBuf) -> Self {
        Self::from_cwd_arc(config, Arc::new(working_directory))
    }

    /// PERF-3 / TASK-0890: zero-clone constructor used by the runner's
    /// `query_data` hot path. The cwd `Arc<PathBuf>` is stored directly so
    /// repeat provider lookups within the same runner share one heap
    /// allocation, mirroring the OWN-2 invariant established for the
    /// parallel-exec path in TASK-0462.
    pub fn from_cwd_arc(config: Arc<Config>, working_directory: Arc<PathBuf>) -> Self {
        Self {
            config,
            data_cache: HashMap::new(),
            in_flight: HashSet::new(),
            working_directory,
            refresh: false,
            #[cfg(feature = "duckdb")]
            db: None,
        }
    }

    /// Read-only accessor for an entry in the data cache (API-9 / TASK-0349).
    ///
    /// Replaces direct field access on `data_cache` so callers can read
    /// previously-provided JSON values without the ability to insert
    /// arbitrary keys outside the [`Context::get_or_provide`] caching
    /// contract.
    #[must_use]
    pub fn cached(&self, key: &str) -> Option<&Arc<serde_json::Value>> {
        self.data_cache.get(key)
    }

    /// Create a context for testing with default config.
    #[cfg(any(test, feature = "test-support"))]
    pub fn test_context(working_directory: PathBuf) -> Self {
        Self::new(Arc::new(Config::default()), working_directory)
    }

    /// Create a context with refresh mode enabled (forces data re-collection).
    pub fn with_refresh(mut self) -> Self {
        self.refresh = true;
        self
    }

    /// Get cached value or compute via provider and cache.
    ///
    /// SEC-38 / TASK-0744: detects re-entrant requests for an in-flight key
    /// (a provider transitively asking for itself, e.g. A → B → A) and
    /// returns [`DataProviderError::Cycle`] instead of recursing into stack
    /// overflow. The in-flight marker is inserted before dispatching to the
    /// provider and removed regardless of success/failure so a provider
    /// that fails does not poison the cache for retry.
    pub fn get_or_provide(
        &mut self,
        key: &str,
        registry: &DataRegistry,
    ) -> Result<Arc<serde_json::Value>, DataProviderError> {
        if let Some(v) = self.data_cache.get(key) {
            return Ok(Arc::clone(v));
        }
        if !self.in_flight.insert(key.to_string()) {
            return Err(DataProviderError::Cycle {
                key: key.to_string(),
            });
        }
        let result = registry.provide(key, self);
        self.in_flight.remove(key);
        let v = Arc::new(result?);
        self.data_cache.insert(key.to_string(), Arc::clone(&v));
        Ok(v)
    }
}
