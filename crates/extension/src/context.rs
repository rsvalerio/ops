//! Per-invocation provider state: the [`Context`] shared with every
//! provider dispatch — config, cwd, cache, cycle guard, deadline, and the
//! feature-gated database handle.
//!
//! ARCH-1 / TASK-2095: split out of `data.rs` — the per-invocation state is
//! a cohesive unit distinct from the registry/schema surface (`crate::data`);
//! the budget type itself lives in `crate::deadline`.

use crate::data::DataRegistry;
#[cfg(feature = "duckdb")]
use crate::db_handle::DuckDbHandle;
use crate::deadline::{configured_provider_budget, Deadline};
use crate::error::DataProviderError;
use ops_core::config::Config;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    /// SEC-33 / TASK-2017: wall-clock budget applied to a provider dispatch
    /// started on this context. `None` means explicitly unbounded.
    provider_budget: Option<Duration>,
    /// SEC-33 / TASK-2017: the deadline of the dispatch currently in flight,
    /// installed by [`DataRegistry::provide`] for the outermost provider and
    /// cleared by the same call. `None` outside a dispatch, or when the
    /// budget is `None`.
    deadline: Option<Deadline>,
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
        let provider_budget = configured_provider_budget(&config);
        Self {
            config,
            data_cache: HashMap::new(),
            in_flight: HashSet::new(),
            working_directory,
            refresh: false,
            provider_budget,
            deadline: None,
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

    /// Clear the in-flight marker set by [`Context::enter_provider`]. Runs in
    /// the dispatch Drop guard, so every exit path — success, error return,
    /// and panic — clears it and a failed provider does not poison later
    /// requests for the same key.
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

    /// SEC-33 / TASK-2017: override the wall-clock budget a provider dispatch
    /// started on this context gets, or pass `None` to opt out of the bound
    /// entirely.
    ///
    /// Defaults to [`crate::deadline::DEFAULT_PROVIDER_BUDGET`]. Like `refresh`, it is set at
    /// construction time rather than exposed as a mutator: a provider that
    /// widened its own budget mid-traversal would be granting itself the
    /// exemption the bound exists to deny.
    #[must_use]
    pub const fn with_provider_budget(mut self, budget: Option<Duration>) -> Self {
        self.provider_budget = budget;
        self
    }

    /// SEC-33 / TASK-2017: the deadline of the dispatch currently in flight,
    /// if any.
    ///
    /// Providers that hand work to something with its own timeout knob (an
    /// external command, a database statement) can use this to size that
    /// timeout so the inner wait cannot outlive the outer budget. Providers
    /// that merely loop should call [`Context::check_deadline`] instead.
    #[must_use]
    pub fn deadline(&self) -> Option<Instant> {
        self.deadline.as_ref().map(Deadline::expires_at)
    }

    /// SEC-33 / TASK-2052: a detached, `Send + Sync` copy of the in-flight
    /// deadline, for a provider whose work happens somewhere a `&Context`
    /// cannot go — a free walker function, or `rust-loc`'s parallel walk,
    /// whose per-entry closure runs on `ignore`'s worker threads.
    ///
    /// Prefer [`Context::check_deadline`] wherever the context itself is in
    /// scope; this exists so that handing the budget to those places does not
    /// degrade the error into an untyped one.
    #[must_use]
    pub fn deadline_handle(&self) -> Option<Deadline> {
        self.deadline.clone()
    }

    /// CONC-9 / TASK-2056: the budget a dispatch started on this context gets,
    /// or `None` when it is explicitly unbounded. Resolved from
    /// `[data] provider_budget_secs` at construction and overridable with
    /// [`Context::with_provider_budget`].
    #[must_use]
    pub const fn provider_budget(&self) -> Option<Duration> {
        self.provider_budget
    }

    /// SEC-33 / TASK-2017: the cooperative cancellation point providers are
    /// required to honour.
    ///
    /// `DataProvider::provide` is synchronous and runs on the caller's
    /// thread, so nothing can preempt it; a provider doing work proportional
    /// to the size of the operator's tree (a directory walk, a per-file read,
    /// a loop over external commands) must therefore poll this itself, once
    /// per unit of work, and propagate the error with `?`:
    ///
    /// ```text
    /// for entry in walker {
    ///     ctx.check_deadline()?;
    ///     // … per-entry work
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// [`DataProviderError::TimedOut`], naming the provider that owns the
    /// budget, once the deadline has passed. Returns `Ok(())` when the
    /// dispatch is unbounded or no deadline is installed.
    pub fn check_deadline(&self) -> Result<(), DataProviderError> {
        self.deadline.as_ref().map_or(Ok(()), Deadline::check)
    }

    /// SEC-33 / TASK-2017: install the deadline for a dispatch of `provider`
    /// if this is the outermost one, and report whether it was installed.
    ///
    /// Nested dispatches inherit the outermost deadline rather than starting
    /// a fresh one: the budget bounds the traversal an operator asked for,
    /// and a provider that composes ten others must not get eleven budgets.
    /// The caller passes the returned flag back to
    /// [`Context::clear_deadline_if_owned`] so only the installer clears it.
    pub(crate) fn begin_deadline(&mut self, provider: &str) -> bool {
        if self.deadline.is_some() {
            return false;
        }
        let Some(budget) = self.provider_budget else {
            return false;
        };
        // A budget large enough to overflow the monotonic clock is a request
        // for no bound at all; install nothing rather than panicking on the
        // addition or wrapping into an instantly-expired deadline.
        let Some(expires_at) = Instant::now().checked_add(budget) else {
            return false;
        };
        self.deadline = Some(Deadline::from_parts(
            provider.to_string(),
            budget,
            expires_at,
        ));
        true
    }

    /// Counterpart to [`Context::begin_deadline`]; a no-op unless this call
    /// installed the deadline. Called on both the success and the failure
    /// path so a failed dispatch does not leave a stale deadline behind to
    /// fail the next one.
    pub(crate) fn clear_deadline_if_owned(&mut self, owned: bool) {
        if owned {
            self.deadline = None;
        }
    }

    /// SEC-33 / TASK-2017: the `TimedOut` error for the in-flight dispatch,
    /// if its deadline has passed. Used by [`DataRegistry::provide`] to
    /// enforce the bound on providers that never poll
    /// [`Context::check_deadline`].
    pub(crate) fn overrun(&self) -> Option<DataProviderError> {
        self.check_deadline().err()
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
