//! Data provider surface: the [`DataProvider`] trait, the [`DataRegistry`],
//! and the schema descriptor types providers publish.
//!
//! ARCH-1 / TASK-2095: the per-invocation state ([`crate::context::Context`])
//! lives in `context.rs`, the dispatch budget ([`crate::deadline::Deadline`])
//! in `deadline.rs`, and the feature-gated database erasure trait
//! ([`crate::db_handle::DuckDbHandle`]) in `db_handle.rs`; this module keeps
//! the registry and schema surface.

use crate::context::Context;
use crate::error::DataProviderError;
use indexmap::IndexMap;
use ops_core::project_identity::AboutFieldDef;

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

    /// Returns the provider registered under `name`, if any.
    ///
    /// Unlike [`DataRegistry::provide`], this neither dispatches the provider
    /// nor consults the context cache — it is a plain map lookup for callers
    /// that only need the registered instance (e.g. to read its schema).
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
    /// The marker is cleared on every exit path — success, error return, and
    /// panic (via the dispatch Drop guard, TASK-2084) — so a provider that
    /// fails or panics does not poison later requests for the same key.
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
        // PATTERN-9 / TASK-2084: teardown lives in the DispatchGuard's Drop,
        // not in fall-through code below — a panicking provider unwinds
        // through this frame, and statements after the call would never run.
        let dispatch = DispatchGuard {
            ctx,
            name,
            owns_deadline,
        };
        let result = provider.provide(&mut *dispatch.ctx);
        // Read the overrun before the guard drops: the deadline is gone
        // afterwards.
        let overrun = dispatch.ctx.overrun();
        drop(dispatch);
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

/// PATTERN-9 / TASK-2084: unwind-safe teardown for one provider dispatch.
/// [`DataRegistry::provide`] sets up in-flight and deadline state before the
/// provider runs and must tear it down afterwards; holding that teardown in
/// `Drop` rather than in statements after the call means a panicking provider
/// — extension-supplied code this workspace cannot audit — cannot leak the
/// in-flight marker (which would make every later request for the key read as
/// a phantom [`DataProviderError::Cycle`]) or strand a deadline this dispatch
/// owned (which would shadow every later dispatch's budget).
struct DispatchGuard<'a> {
    ctx: &'a mut Context,
    name: &'a str,
    owns_deadline: bool,
}

impl Drop for DispatchGuard<'_> {
    fn drop(&mut self) {
        self.ctx.clear_deadline_if_owned(self.owns_deadline);
        self.ctx.exit_provider(self.name);
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
    /// the documented expectations of [`DataRegistry::take_duplicate_inserts`]
    /// audit-trail consumers and aligning with the insertion-order
    /// policy of [`crate::CommandRegistry`].
    fn into_iter(self) -> Self::IntoIter {
        self.providers.into_iter()
    }
}
