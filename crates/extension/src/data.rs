//! Data provider system: `DataProvider` trait, `DataRegistry`, Context, `DuckDbHandle`.

use crate::error::DataProviderError;
use indexmap::IndexMap;
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
///
/// # Why `&'static str`?
///
/// API-2 / TASK-1135: `name`, `type_name`, and `description` are intentionally
/// `&'static str` rather than `String` or `Cow<'static, str>`. Field
/// descriptors are part of an extension's *compile-time identity* — they
/// describe a stable schema surface that tooling (`cargo ops data info`,
/// about-card rendering) reads to document the extension. In every
/// in-tree usage and in the [`crate::data_field!`] macro the values are
/// string literals baked into the binary; making the type owned would
/// imply a per-call allocation profile that does not exist in practice.
///
/// **For runtime-generated field descriptions**: do *not* reach for
/// `Box::leak`. Instead, build your provider so that schemas are produced
/// by `match`-ing over a closed enum of supported field shapes whose
/// descriptions are static literals, or change [`DataProvider::schema`]
/// to compute the dynamic data through a different surface (e.g. a
/// separate `Vec<String>`-shaped accessor). If a future use case
/// genuinely needs runtime-owned strings, migrate the type to
/// `Cow<'static, str>` rather than leaking — but coordinate with the
/// extension framework owner because every implementer's `data_field!`
/// invocations and `schema()` returns must move together.
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
    ///
    /// All three arguments are `&'static str`; see the type-level docs for
    /// the rationale and guidance on runtime-generated descriptions.
    #[must_use]
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
///
/// API-2 / TASK-1135: `description` is `&'static str` for the same reason
/// described on [`DataField`] — schema text is a compile-time identity for
/// the provider. See [`DataField`]'s type-level docs for guidance when a
/// caller needs runtime-generated text.
#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub struct DataProviderSchema {
    pub description: &'static str,
    pub fields: Vec<DataField>,
}

impl DataProviderSchema {
    /// Construct a [`DataProviderSchema`].
    ///
    /// `description` is `&'static str`; see [`DataField`] for the rationale.
    #[must_use]
    pub const fn new(description: &'static str, fields: Vec<DataField>) -> Self {
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
///
/// # Why no `Debug` supertrait
///
/// TRAIT-4 / TASK-1879: adding `Debug` as a supertrait was considered and
/// **rejected**. It would be a breaking change for every out-of-tree
/// implementer for a benefit that is already covered: [`DataRegistry`]'s
/// `Debug` impl names providers by their registered key, which is the
/// identity every diagnostic in this crate (the duplicate-insert breadcrumb,
/// `provider_names`, `DataProviderError::NotFound`) already reports. A
/// concrete provider type name would add nothing a key does not, and
/// providers commonly hold connection handles and credentials whose derived
/// `Debug` output is exactly what should not reach a log. Implementers who
/// want a representation may derive `Debug` on their own type; nothing here
/// prevents it.
pub trait DataProvider: Send + Sync {
    /// Returns the unique name of this data provider.
    ///
    /// This name is used to register and query the provider via `DataRegistry`.
    fn name(&self) -> &'static str;

    /// Provides data, potentially using context for caching or configuration.
    ///
    /// Implementations may:
    /// - Use `ctx.db()` to query an attached database handle
    /// - Use `ctx.config()` to access configuration
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
    /// - [`DataProviderError::Cycle`] (SEC-38 / TASK-0744, TASK-1865) is
    ///   returned by [`DataRegistry::provide`] — and therefore by
    ///   [`Context::get_or_provide`], which dispatches through it — when a
    ///   provider transitively re-requests a key already in flight.
    ///   Implementations that compose other providers should propagate this
    ///   variant rather than swallowing it, so the cycle surfaces at the
    ///   originating call site.
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

/// Registry of provider name → `DataProvider`.
///
/// API-9 / TASK-1179: backed by [`IndexMap`] so iteration (including the
/// public [`IntoIterator`] impl) yields entries in registration order. The
/// previous `HashMap` exposed hashbrown's randomised iteration order to
/// downstream consumers, which silently surfaced as non-deterministic
/// warning ordering for the `take_duplicate_inserts` audit trail and
/// non-reproducible CLI output. `provider_names` continues to return a
/// sorted view for surfaces that prefer alphabetical ordering; the
/// untyped iteration order is now stable and matches the
/// insertion-order policy of [`crate::CommandRegistry`].
#[derive(Default)]
pub struct DataRegistry {
    providers: IndexMap<String, Box<dyn DataProvider>>,
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
    /// CL-5 / TASK-0661, CL-3 / TASK-1872: this registry is
    /// **first-write-wins**. The rationale and the contrast with
    /// [`crate::CommandRegistry::insert`]'s last-write-wins policy are
    /// documented once, in [`crate::registry_duplicate_policy`]; do not
    /// restate them here or on the sibling method.
    ///
    /// CL-5 / TASK-0756: the previous implementation also fired a
    /// `debug_assert!(false)` on collision, which weaponised tests against
    /// any in-extension duplicate (the wiring layer's per-extension scratch
    /// registry would panic instead of letting the wiring code aggregate
    /// the warning). The audit-trail mechanism replaces that panic so
    /// in-extension duplicates surface as a single warning emitted from one
    /// place rather than as a bespoke panic.
    ///
    /// API-9 / TASK-1067: when a duplicate is detected, the incoming
    /// `Box<dyn DataProvider>` is dropped at the end of this call (the first
    /// registration wins) and a `tracing::debug!` breadcrumb is emitted at
    /// the drop site naming the rejected provider so that any constructor
    /// side effects (DB handles, file descriptors) opened by the dropped
    /// provider are at least observable in logs. The aggregated
    /// `tracing::warn!` emitted by the CLI wiring layer via
    /// [`take_duplicate_inserts`](Self::take_duplicate_inserts) remains the
    /// aggregated user-facing signal; the debug breadcrumb here is the
    /// finer-grained drop-site trace.
    ///
    /// CL-3 / TASK-1872: the outcome is also returned. Previously `register`
    /// returned `()`, so from the call site a rejected registration was
    /// indistinguishable from an accepted one and the *only* failure channel
    /// was an audit `Vec` some later, unrelated caller had to remember to
    /// drain — a precondition the compiler cannot check, and one that had
    /// already been missed once on the sibling registry (DUP-3 / TASK-1225).
    /// Returning the rejected provider mirrors
    /// [`crate::CommandRegistry::insert`]'s shape and makes ignoring the
    /// outcome an explicit `let _ = …` rather than the invisible default.
    ///
    /// Returns `None` when `name` was free and the provider was installed,
    /// or `Some(provider)` handing back the rejected value when a provider
    /// was already registered under `name`.
    #[must_use = "a returned provider was rejected as a duplicate and is about to be dropped; \
                  bind it with `let _ = …` to accept that, or keep it"]
    pub fn register(
        &mut self,
        name: impl Into<String>,
        provider: Box<dyn DataProvider>,
    ) -> Option<Box<dyn DataProvider>> {
        let name = name.into();
        // PATTERN-3 / TASK-1489: route through `IndexMap::entry` so the happy
        // path consults the inner map exactly once, mirroring the sibling
        // `CommandRegistry::insert` (CL-5 / TASK-0756) which was previously
        // migrated under PATTERN-3 / TASK-0753. READ-4 / TASK-1881: the cost
        // profile that buys is one hash probe instead of two on the happy
        // path; the duplicate path pays a clone of the key already stored in
        // the map, because `entry` consumed the incoming `name`.
        match self.providers.entry(name) {
            indexmap::map::Entry::Occupied(occupied) => {
                // SEC-21 / TASK-1226: `name` is `impl Into<String>` and may be
                // runtime-generated by an extension reading external data
                // (e.g. a name pulled from a manifest). Format via Debug so
                // newlines / ANSI sequences cannot forge log entries. The
                // sister `dropped_provider_reports_name` flows from
                // `DataProvider::name()`, which is `&'static str` for every
                // provider in this codebase, so the Display formatter is
                // safe there.
                tracing::debug!(
                    provider_name = ?occupied.key(),
                    dropped_provider_reports_name = %provider.name(),
                    "DataRegistry::register rejecting duplicate provider (first-write-wins); the incoming Box<dyn DataProvider> is returned to the caller"
                );
                self.duplicate_inserts.push(occupied.key().clone());
                Some(provider)
            }
            indexmap::map::Entry::Vacant(vacant) => {
                vacant.insert(provider);
                None
            }
        }
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
        self.providers.get(name).map(std::convert::AsRef::as_ref)
    }

    /// Returns the registered provider names in sorted order.
    ///
    /// API-3 / TASK-0996: previously paired with a `provider_names_iter`
    /// method whose name promised zero-allocation streaming but whose body
    /// collected into an intermediate `Vec` to perform the sort. The two
    /// shapes paid the same cost while misleading callers about the
    /// allocation profile. Collapsed to a single `Vec`-returning accessor
    /// — sorting registered provider names *requires* materialising them,
    /// so the type signature now matches the cost.
    pub fn provider_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.providers.keys().map(String::as_str).collect();
        names.sort_unstable();
        names
    }

    /// Returns schemas for all providers that have non-empty descriptions.
    #[must_use]
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
            .map(DataProvider::about_fields)
            .unwrap_or_default()
    }

    /// Dispatch to the provider registered under `name`.
    ///
    /// SEC-38 / TASK-1865: the re-entrancy guard lives **here**, at the single
    /// dispatch point, rather than in the caching wrapper
    /// [`Context::get_or_provide`]. Previously the `in_flight` marker was set
    /// only by `get_or_provide`, so a provider composing others through this
    /// method (or through a `&dyn DataProvider` obtained from
    /// [`DataRegistry::get`]) re-entered the provider graph unguarded and an
    /// A -> B -> A cycle recursed until stack overflow — an abort, not a
    /// catchable error. Both public entry points now cross this function, so
    /// the guard cannot be bypassed by picking the other one.
    ///
    /// The marker is cleared on both the success and the failure path so a
    /// provider that fails does not poison later requests for the same key.
    ///
    /// # Errors
    ///
    /// [`DataProviderError::NotFound`] if no provider is registered under
    /// `name`, [`DataProviderError::Cycle`] if a provider for `name` is
    /// already executing on `ctx`, or whatever error the provider itself
    /// returns.
    pub fn provide(
        &self,
        name: &str,
        ctx: &mut Context,
    ) -> Result<serde_json::Value, DataProviderError> {
        let provider = self
            .providers
            .get(name)
            .ok_or_else(|| DataProviderError::not_found(name))?;
        ctx.enter_provider(name)?;
        let result = provider.provide(ctx);
        ctx.exit_provider(name);
        result
    }
}

/// TRAIT-4 / TASK-1879: hand-written because `Box<dyn DataProvider>` is not
/// `Debug`, which is a reason to write the impl rather than to have none —
/// without it no downstream type holding a `DataRegistry` can derive `Debug`,
/// and the omission propagates outward. Prints the provider names in
/// registration order plus any audit-trail entries not yet drained.
impl std::fmt::Debug for DataRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataRegistry")
            .field("providers", &self.providers.keys())
            .field("duplicate_inserts", &self.duplicate_inserts)
            .finish()
    }
}

impl IntoIterator for DataRegistry {
    type Item = (String, Box<dyn DataProvider>);
    type IntoIter = indexmap::map::IntoIter<String, Box<dyn DataProvider>>;
    /// API-9 / TASK-1179: yields entries in registration order, matching
    /// the documented expectations of [`take_duplicate_inserts`]
    /// audit-trail consumers and aligning with the insertion-order
    /// policy of [`crate::CommandRegistry`].
    fn into_iter(self) -> Self::IntoIter {
        self.providers.into_iter()
    }
}

/// Erasure trait for the `DuckDb` handle so that extension.rs does not depend
/// on duckdb types.
///
/// # Downcast contract
///
/// The only concrete type stored behind `Arc<dyn DuckDbHandle>` in production
/// code is `ops_duckdb::DuckDb`.
///
/// **Reborrow as `&dyn DuckDbHandle` first.** The blanket impl below covers
/// every `'static + Send + Sync` type, and `Arc<dyn DuckDbHandle>` is one of
/// them — so method resolution on an `Arc` (or `&Arc`) receiver matches the
/// blanket impl *for the smart pointer* before it derefs, and `as_any()`
/// hands back the erased `Arc` instead of the handle. Every downcast from it
/// then returns `None`. Downcast call sites should:
///
/// ```text
/// let erased: &dyn DuckDbHandle = handle.as_ref();
/// let db: Option<&ops_duckdb::DuckDb> = erased
///     .as_any()
///     .downcast_ref::<ops_duckdb::DuckDb>();
/// ```
///
/// or use the typed convenience helper [`ops_duckdb::get_db`] which performs
/// the downcast and returns `Option<&DuckDb>`. New consumers should prefer
/// `get_db` over calling `as_any` directly to avoid coupling on the concrete
/// trait method (FN-9).
///
/// # Implementing
///
/// TRAIT-9 / TASK-1227: a blanket impl provides the canonical `as_any`
/// body for every `'static + Send + Sync` type, so implementers cannot
/// accidentally (or maliciously) return a wrong reference like `&()`
/// that would silently break every downcast. The previous shape relied
/// on a doc-only "the implementer must return self" contract; that is
/// now compile-time-enforced — implementers do not (and cannot) supply
/// their own `as_any` body. Any `'static + Send + Sync` type
/// automatically satisfies `DuckDbHandle`, no explicit `impl` block
/// required at the call site.
#[cfg(feature = "duckdb")]
pub trait DuckDbHandle: std::any::Any + Send + Sync {
    /// Return the handle as `&dyn Any` so callers can downcast to the
    /// concrete type. The blanket impl below supplies the canonical
    /// body (`self`); see trait-level docs for the supported concrete
    /// type and the preferred typed accessor.
    fn as_any(&self) -> &dyn std::any::Any;
}

#[cfg(feature = "duckdb")]
impl<T: std::any::Any + Send + Sync> DuckDbHandle for T {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

/// Per-invocation context shared with data providers.
///
/// API-9 / TASK-0349: marked `#[non_exhaustive]` so that adding a field is
/// not a `SemVer` break for downstream providers. `data_cache` is no longer
/// `pub`; reads go through [`Context::cached`] and writes go through
/// [`Context::get_or_provide`] so callers cannot bypass the
/// caching/provider contract by inserting raw values directly.
///
/// ARCH-9 / TASK-1874: every remaining field is private too. Providers
/// receive `&mut Context`, so a public field is a mutation channel one
/// provider can use to change what its *siblings* observe later in the same
/// traversal — `refresh` flips the cache-bypass semantics for every
/// subsequent `get_or_provide` on this context, and `working_directory`
/// re-points path resolution for every provider that runs afterwards (a
/// confused deputy within a single command invocation). Reads go through
/// [`Context::config`], [`Context::working_directory`],
/// [`Context::is_refreshing`] and [`Context::db`]; the only mutators are the
/// constructors, [`Context::with_refresh`], [`Context::attach_db`] and
/// [`Context::clear_provider_results`].
#[non_exhaustive]
pub struct Context {
    config: Arc<Config>,
    data_cache: HashMap<String, Arc<serde_json::Value>>,
    /// SEC-38 / TASK-0744: keys whose providers are currently executing on
    /// this context. Inserted before dispatching in
    /// [`DataRegistry::provide`] and removed on the way out, so a provider
    /// that transitively re-requests its own key surfaces as
    /// `DataProviderError::Cycle` instead of recursing until stack overflow.
    in_flight: HashSet<String>,
    /// PERF-3 / TASK-0890: stored as `Arc<PathBuf>` so the runner can hand
    /// out cheap `Arc::clone`s on every `query_data` invocation instead of
    /// deep-cloning the inner path. Read it as a `&Path` via
    /// [`Context::working_directory`], or share the allocation via
    /// [`Context::working_directory_arc`].
    working_directory: Arc<PathBuf>,
    /// When true, data providers should re-collect data instead of using cached/persisted results.
    refresh: bool,
    #[cfg(feature = "duckdb")]
    db: Option<Arc<dyn DuckDbHandle>>,
}

impl Context {
    #[must_use]
    pub fn new(config: Arc<Config>, working_directory: PathBuf) -> Self {
        Self::from_cwd_arc(config, Arc::new(working_directory))
    }

    /// PERF-3 / TASK-0890: zero-clone constructor used by the runner's
    /// `query_data` hot path. The cwd `Arc<PathBuf>` is stored directly so
    /// repeat provider lookups within the same runner share one heap
    /// allocation, mirroring the OWN-2 invariant established for the
    /// parallel-exec path in TASK-0462.
    #[must_use]
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

    /// The configuration this invocation was started with.
    ///
    /// ARCH-9 / TASK-1874: read-only. Swapping the config mid-traversal would
    /// change what every later provider sees, so the field is set once by the
    /// constructors.
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Share the config allocation with a provider that needs to hold on to
    /// it beyond the borrow of `&Context`.
    #[must_use]
    pub const fn config_arc(&self) -> &Arc<Config> {
        &self.config
    }

    /// The directory paths in this invocation resolve against.
    ///
    /// ARCH-9 / TASK-1874: read-only. Re-pointing it mid-traversal would make
    /// providers composed later read from a directory the caller never asked
    /// for.
    #[must_use]
    pub fn working_directory(&self) -> &std::path::Path {
        self.working_directory.as_path()
    }

    /// Share the cwd allocation (PERF-3 / TASK-0890) without deep-cloning the
    /// inner [`PathBuf`].
    #[must_use]
    pub const fn working_directory_arc(&self) -> &Arc<PathBuf> {
        &self.working_directory
    }

    /// Whether providers should re-collect instead of serving cached or
    /// persisted results.
    ///
    /// ARCH-9 / TASK-1874: read-only for providers. Set it at construction
    /// time via [`Context::with_refresh`]; a provider that assigned to it
    /// would change caching behaviour for every sibling that ran afterwards.
    #[must_use]
    pub const fn is_refreshing(&self) -> bool {
        self.refresh
    }

    /// The attached database handle, if the duckdb extension has installed
    /// one on this context.
    #[cfg(feature = "duckdb")]
    #[must_use]
    pub fn db(&self) -> Option<&Arc<dyn DuckDbHandle>> {
        self.db.as_ref()
    }

    /// Attach (or replace) the database handle.
    ///
    /// ARCH-9 / TASK-1874: unlike `refresh` and `working_directory`, `db` is
    /// genuinely provider-assigned — the duckdb extension opens the handle
    /// lazily on first use and installs it here so sibling providers reuse
    /// the same connection. That is a *capability being added*, not a
    /// reinterpretation of what earlier providers already did, so it keeps a
    /// mutator. It is a named method rather than a public field so the
    /// assignment is greppable and cannot be confused with the read-only
    /// fields around it.
    #[cfg(feature = "duckdb")]
    pub fn attach_db(&mut self, db: Arc<dyn DuckDbHandle>) {
        self.db = Some(db);
    }

    /// SEC-38 / TASK-1865: mark `key` as executing on this context.
    ///
    /// Returns [`DataProviderError::Cycle`] when a provider for `key` is
    /// already in flight — the re-entrancy that would otherwise recurse to a
    /// stack overflow. Called by [`DataRegistry::provide`], the single
    /// dispatch point, so no public entry point can skip it.
    pub(crate) fn enter_provider(&mut self, key: &str) -> Result<(), DataProviderError> {
        if self.in_flight.insert(key.to_string()) {
            Ok(())
        } else {
            Err(DataProviderError::Cycle {
                key: key.to_string(),
            })
        }
    }

    /// Clear the in-flight marker set by [`Context::enter_provider`]. Called
    /// on both the success and the failure path so a failed provider does not
    /// poison later requests for the same key.
    pub(crate) fn exit_provider(&mut self, key: &str) {
        self.in_flight.remove(key);
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
    #[must_use]
    pub fn test_context(working_directory: PathBuf) -> Self {
        Self::new(Arc::new(Config::empty()), working_directory)
    }

    /// Create a context with refresh mode enabled (forces data re-collection).
    #[must_use]
    pub const fn with_refresh(mut self) -> Self {
        self.refresh = true;
        self
    }

    /// Get cached value or compute via provider and cache.
    ///
    /// SEC-38 / TASK-0744, TASK-1865: re-entrant requests for an in-flight key
    /// (a provider transitively asking for itself, e.g. A → B → A) surface as
    /// [`DataProviderError::Cycle`] instead of recursing into stack overflow.
    /// The guard itself lives in [`DataRegistry::provide`] — the single
    /// dispatch point — so it also covers callers that reach a provider
    /// without going through this cache wrapper. This method is the cache
    /// fast-path plus a call into that dispatch.
    ///
    /// ERR-1 / TASK-1170: when `self.refresh` is true the cache fast-path is
    /// bypassed and the provider is re-invoked, then the fresh value
    /// overwrites the cached entry. Without this, `Context::with_refresh()`
    /// (and any caller setting `refresh = true`) would silently serve stale
    /// cached values for any key already populated on this context — a
    /// regression that became user-visible once TASK-0993 folded the cache
    /// onto the persistent runner `Context`, which lives across repeat
    /// queries within a single runner lifetime.
    ///
    /// # Errors
    ///
    /// Whatever the underlying provider returns; see [`DataRegistry::provide`].
    /// A cache hit (when `refresh` is false) cannot fail.
    pub fn get_or_provide(
        &mut self,
        key: &str,
        registry: &DataRegistry,
    ) -> Result<Arc<serde_json::Value>, DataProviderError> {
        if !self.refresh {
            if let Some(v) = self.data_cache.get(key) {
                return Ok(Arc::clone(v));
            }
        }
        let value = registry.provide(key, self)?;
        let v = Arc::new(value);
        self.data_cache.insert(key.to_string(), Arc::clone(&v));
        Ok(v)
    }

    /// ARCH-9 / TASK-1128: drop every cached provider result and any
    /// in-flight markers. The runner calls this from
    /// `register_data_providers` so swapping in a new [`DataRegistry`] does
    /// not leave callers reading values produced by the previous registry's
    /// providers (or by a different implementation registered under the same
    /// name).
    pub fn clear_provider_results(&mut self) {
        self.data_cache.clear();
        self.in_flight.clear();
    }
}

/// TRAIT-4 / TASK-1879: hand-written because the optional `Arc<dyn
/// DuckDbHandle>` is not `Debug`. Prints cache and in-flight **keys only** —
/// never the cached values, which are arbitrary provider output and may be
/// large or carry data that has no business in a panic message. Keys are
/// sorted so the rendering is deterministic across runs despite the backing
/// `HashMap`/`HashSet` iteration order.
// `config` is deliberately omitted: it is a large tree whose own `Debug` would
// dominate every rendering of a `Context`, and it is invariant for the
// lifetime of the context, so it tells a reader nothing about *this* traversal.
// Read it through `Context::config()` when it is what you actually want.
#[allow(clippy::missing_fields_in_debug)]
impl std::fmt::Debug for Context {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut cached: Vec<&str> = self.data_cache.keys().map(String::as_str).collect();
        cached.sort_unstable();
        let mut in_flight: Vec<&str> = self.in_flight.iter().map(String::as_str).collect();
        in_flight.sort_unstable();
        let mut s = f.debug_struct("Context");
        s.field("working_directory", &self.working_directory)
            .field("refresh", &self.refresh)
            .field("cached_keys", &cached)
            .field("in_flight", &in_flight);
        #[cfg(feature = "duckdb")]
        s.field("db", &self.db.is_some());
        s.finish()
    }
}
