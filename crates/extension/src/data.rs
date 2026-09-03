//! Data provider system: `DataProvider` trait, `DataRegistry`, Context, `DuckDbHandle`.

use crate::error::DataProviderError;
use indexmap::IndexMap;
use ops_core::config::Config;
use ops_core::project_identity::AboutFieldDef;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// SEC-33 / TASK-2017: default wall-clock budget for one provider dispatch.
///
/// [`DataProvider::provide`] is synchronous, so this is a *cooperative*
/// bound, not a preemptive one: it is enforced by
/// [`Context::check_deadline`] inside providers that poll it and, for
/// providers that do not, by [`DataRegistry::provide`] refusing to return a
/// value produced after the deadline. It exists so that no provider dispatch
/// is unbounded by construction; it cannot interrupt a thread already blocked
/// in a syscall.
///
/// The value is deliberately generous. Providers here range from a
/// sub-millisecond `Cargo.toml` read to `cargo llvm-cov` over a whole
/// workspace, and a budget tight enough to be interesting for the first would
/// turn the second into a spurious failure. Twenty minutes is an *upper bound
/// on a stall*, not a latency target. Callers that know their own tolerance
/// narrow it with [`Context::with_provider_budget`].
///
/// CONC-9 / TASK-2068: this used to carry the ordering requirement as prose —
/// the budget had to stay **above every subprocess timeout a provider can wait
/// on**, or it would fire first and report a run still within its own limit as
/// a failure. TASK-2056 made the budget operator-configurable
/// (`[data] provider_budget_secs`), which put that invariant at the mercy of a
/// config file no test could police. The binding subprocess wait,
/// `ops-test-coverage`'s `CARGO_LLVM_COV_TIMEOUT` (15 minutes), now sizes
/// itself from [`Context::deadline`] instead, so the two agree by construction
/// at whatever value this budget takes and the ordering no longer has to be
/// maintained by hand.
///
/// The twenty minutes still buy headroom over that fifteen for the workspace
/// walk and parsing either side of the subprocess. A provider that hands work
/// to something with its own timeout knob should follow the coverage provider
/// and size it from [`Context::deadline`] rather than assume a floor here.
pub const DEFAULT_PROVIDER_BUDGET: Duration = Duration::from_mins(20);

/// The budget installed for the provider dispatch currently in flight.
///
/// Held by [`Context`] for the duration of the outermost
/// [`DataRegistry::provide`] call and inherited by every provider that one
/// composes, so a provider graph cannot multiply its budget by nesting.
///
/// SEC-33 / TASK-2052: public and detachable ([`Context::deadline_handle`])
/// because the providers that most need to poll it are tree walkers whose
/// walk lives in a free function — sometimes, as in `rust-loc`, one that runs
/// on worker threads that cannot borrow `&Context` at all. A `Deadline` is
/// `Clone + Send + Sync` and carries everything [`Deadline::check`] needs to
/// build the same error [`Context::check_deadline`] would, so threading it
/// into a walker does not weaken the failure an operator sees.
#[derive(Debug, Clone)]
pub struct Deadline {
    /// The provider that owns the budget — the outermost one, which is the
    /// one an operator asked for and the one worth naming in the failure.
    provider: String,
    budget: Duration,
    expires_at: Instant,
}

impl Deadline {
    /// When this dispatch's budget runs out.
    #[must_use]
    pub const fn expires_at(&self) -> Instant {
        self.expires_at
    }

    /// Whether the budget has already run out.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Instant::now() >= self.expires_at
    }

    /// The cooperative cancellation point, for code that holds a detached
    /// deadline rather than a `&Context`.
    ///
    /// # Errors
    ///
    /// [`DataProviderError::TimedOut`], naming the provider that owns the
    /// budget, once the deadline has passed.
    pub fn check(&self) -> Result<(), DataProviderError> {
        if self.is_expired() {
            return Err(DataProviderError::TimedOut {
                provider: self.provider.clone(),
                budget: self.budget,
            });
        }
        Ok(())
    }
}

/// CONC-9 / TASK-2056: resolve the dispatch budget an operator configured,
/// falling back to [`DEFAULT_PROVIDER_BUDGET`].
///
/// `[data] provider_budget_secs = 0` is the documented opt-out and maps to
/// `None` (unbounded), which is the one value that must not be confused with
/// "unset": a zero-length budget would time every dispatch out instantly, so
/// reading it literally would turn a knob meant to *remove* the bound into
/// one that makes every provider fail.
const fn configured_provider_budget(config: &Config) -> Option<Duration> {
    match config.data.provider_budget_secs {
        None => Some(DEFAULT_PROVIDER_BUDGET),
        Some(0) => None,
        Some(secs) => Some(Duration::from_secs(secs)),
    }
}

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
///
/// # Why [`DataProvider::provide`] stays synchronous and on the caller's thread
///
/// SEC-33 / TASK-2052 asked this explicitly, so the answer is recorded here
/// rather than left implicit in the shape of the trait. **Decision: it stays
/// synchronous, and the bound stays cooperative.** Both alternatives were
/// considered and rejected:
///
/// - *Make the trait `async`.* It is a breaking change for every in-tree and
///   out-of-tree implementer, and it pulls an async runtime into
///   `ops-extension`, which today has none. What it buys is nothing on its
///   own: the tree walkers are CPU- and syscall-bound, not `await`-bound, so
///   they would still need exactly the per-entry `check_deadline` this task
///   adds in order to yield. Async moves the cancellation point; it does not
///   create one.
/// - *Run the dispatch on a worker thread and time-out the join.* This bounds
///   the *caller* but not the *work*: a thread blocked in `readdir` on a
///   wedged mount cannot be cancelled in Rust, so the stalled thread is
///   leaked, still holding the provider's resources, and the process cannot
///   exit while it lives. It converts a visible stall into an invisible one,
///   and would make the fix a lie.
///
/// So the residual risk the finding names — a provider already blocked in a
/// syscall — is accepted rather than solved. It is bounded in practice by the
/// per-entry check (the walk stops at the *next* entry) and reported by
/// [`DataRegistry::provide`], which refuses to return a value produced after
/// the deadline. Revisit only if a provider appears whose single unit of work
/// can itself outlast a budget.
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
    /// - [`DataProviderError::TimedOut`] (SEC-33 / TASK-2017) when the
    ///   dispatch outlives the budget on the context.
    ///
    /// # Honouring the deadline
    ///
    /// This method is synchronous and runs on the caller's thread, so nothing
    /// can interrupt it. An implementation whose cost scales with the
    /// operator's tree — a directory walk, a per-file read, a loop over
    /// external commands — must therefore call
    /// [`Context::check_deadline`] once per unit of work and propagate the
    /// error with `?`. Implementations that do not are still bounded at the
    /// dispatch point, but only after the fact: the caller gets `TimedOut`
    /// instead of a late value, and the stall itself still happened.
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
    /// API-9 / TASK-1067: when a duplicate is detected, the first
    /// registration wins and the incoming `Box<dyn DataProvider>` is handed
    /// back to the caller as `Some(provider)` rather than dropped here — it
    /// is dropped only if the caller discards the return value. A
    /// `tracing::debug!` breadcrumb is emitted at the rejection site naming
    /// the rejected provider so that any constructor side effects (DB
    /// handles, file descriptors) opened by a provider the caller then drops
    /// are at least observable in logs. The aggregated
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
    /// SEC-33 / TASK-2017: the wall-clock bound lives here too, for the same
    /// reason the re-entrancy guard does — it is the one place both public
    /// entry points cross, so no provider can acquire an unbounded dispatch
    /// by being reached through the other one, and no new provider has to
    /// remember to opt in.
    ///
    /// The bound is cooperative. A synchronous `provide` running on this
    /// thread cannot be preempted, so the deadline installed on `ctx` is what
    /// providers doing long chunked work poll via
    /// [`Context::check_deadline`]. For providers that do not poll, this
    /// function still refuses to return a value produced after the deadline:
    /// an over-budget `Ok` becomes [`DataProviderError::TimedOut`] rather
    /// than a silent late success, which is what keeps the overrun visible in
    /// an operator log instead of only in the wall clock. What it cannot do
    /// is shorten the stall itself — see TASK-2052.
    ///
    /// # Errors
    ///
    /// [`DataProviderError::NotFound`] if no provider is registered under
    /// `name`, [`DataProviderError::Cycle`] if a provider for `name` is
    /// already executing on `ctx`, whatever error the provider itself returns,
    /// or [`DataProviderError::TimedOut`] if a provider that would otherwise
    /// have *succeeded* ran past its budget.
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
        let owns_deadline = ctx.begin_deadline(name);
        let result = provider.provide(ctx);
        // Read the overrun before clearing: the deadline is gone afterwards.
        let overrun = ctx.overrun();
        ctx.clear_deadline_if_owned(owns_deadline);
        ctx.exit_provider(name);
        match (result, overrun) {
            // A provider error is the specific answer and is returned
            // verbatim, whether or not the dispatch also ran past its budget.
            // That covers the provider that polled `check_deadline` and
            // already built an identical `TimedOut` naming the same owner, and
            // equally the command or I/O failure that happens to have taken
            // too long: reporting the overrun instead would replace the reason
            // the dispatch failed with the fact that it was slow.
            (Err(err), _) => Err(err),
            (Ok(_), Some(timed_out)) => Err(timed_out),
            (Ok(value), None) => Ok(value),
        }
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
/// them — so wherever this trait is in scope, method resolution on an `Arc`
/// (or `&Arc`) receiver matches the blanket impl *for the smart pointer*
/// before it derefs, and `as_any()` hands back the erased `Arc` instead of the
/// handle. Every downcast from it then returns `None`.
///
/// SEC-38 / TASK-2018: whether it misresolves depends on the *importing
/// module*, which is what makes it dangerous. A module that never names
/// `DuckDbHandle` in a `use` has no blanket-impl candidate in scope, so a bare
/// `handle.as_any()` derefs through to the trait object's own method and
/// downcasts correctly — until someone adds the import, at which point every
/// downcast in that module silently starts returning `None` with no compile
/// error and no runtime error. Do not rely on the import list. Downcast call
/// sites should:
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
///
/// ## Why the blanket impl is not narrowed (SEC-38 / TASK-2018)
///
/// Narrowing it so an `Arc` receiver fails to compile instead of silently
/// misresolving was considered and **rejected**. The three shapes that would
/// achieve it each cost more than the hazard:
///
/// - `impl DuckDbHandle for DuckDb` only — `ops-extension` exists precisely so
///   it does not depend on `ops-duckdb`; this inverts that dependency.
/// - A sealed marker supertrait — restores the doc-only "return `self`"
///   contract this impl replaced (TRAIT-9 / TASK-1227), since every implementer
///   would again write its own `as_any` body.
/// - A negative impl for `Arc<T>` — not available on stable Rust.
///
/// The blanket impl stays, and the misresolution is contained at the two places
/// that matter instead: the mandated reborrow above, and `ops_duckdb::get_db` /
/// `try_provide_from_db`, which perform the reborrow for every consumer and
/// carry regression tests that call them with the trait deliberately in scope.
/// New consumers should call those helpers rather than `as_any` directly.
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

    /// SEC-33 / TASK-2017: override the wall-clock budget a provider dispatch
    /// started on this context gets, or pass `None` to opt out of the bound
    /// entirely.
    ///
    /// Defaults to [`DEFAULT_PROVIDER_BUDGET`]. Like `refresh`, it is set at
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
        self.deadline.as_ref().map(|d| d.expires_at)
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
        self.deadline = Some(Deadline {
            provider: provider.to_string(),
            budget,
            expires_at,
        });
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
