//! Command execution engine: exec and composite commands, `RunnerEvent` stream.
//!
//! # Architecture
//!
//! `CommandRunner` is the central orchestrator. Concerns are split across
//! sibling modules so this file stays focused on construction, accessors,
//! and the top-level [`CommandRunner::run`] dispatch:
//!
//! - [`build`] — building a tokio `Command` from an `ExecCommandSpec`,
//!   workspace-escape policy.
//! - [`events`] — `RunnerEvent` enum + `PlanLifecycle` bookend.
//! - [`exec`] — spawning a single child, capturing/streaming output.
//! - [`resolve`] — config / stack / extension lookup, alias resolution,
//!   composite expansion.
//! - [`sequential`] — `run_plan` / `run_plan_raw` / `run_raw` orchestration.
//! - [`parallel`] — bounded mpsc channel, fail-fast cancellation, `JoinSet`
//!   collection.
//! - [`process_group`] — process-group ownership so cancellation reaches a
//!   step's whole descendant tree, not just its direct child.
//! - [`secret_patterns`] — env-value secret heuristics.
//!
//! ## Command Resolution Priority
//!
//! Commands are resolved in this order (highest to lowest priority):
//! 1. **Config commands**: From `.ops.toml` or internal defaults
//! 2. **Stack commands**: Language/stack-specific defaults (e.g., `cargo fmt` for Rust)
//! 3. **Extension commands**: Commands registered by extensions
//!
//! Why one struct and not three? All concerns share the same config/cwd
//! context, the data cache must span resolution and execution, and the
//! public API is stable and well-tested.

mod abort;
mod build;
mod builtins;
mod events;
mod exec;
mod parallel;
mod process_group;
mod resolve;
mod results;
mod secret_patterns;
mod sequential;

pub use build::CwdEscapePolicy;
use build::WorkspaceCanonicalCache;
pub use events::{OutputLine, RunnerEvent};
pub use results::StepResult;
pub use secret_patterns::is_sensitive_env_key;
pub use secret_patterns::looks_like_secret_value;

/// Shared "id not found in any store" failure.
///
/// [`ResolveExecError`] and [`ExpandError`] both wrap this struct for their
/// `Unknown` variant, so the message lives in one place and a caller can
/// convert between the parent enums via `#[from]` without reconstructing the
/// inner string.
#[derive(Debug, thiserror::Error, PartialEq, Eq, Clone)]
#[error("unknown command: {0}")]
pub struct UnknownCommand(pub String);

impl UnknownCommand {
    /// Convenience constructor accepting any borrowed-string-ish input so
    /// call sites stay terse: `UnknownCommand::new(id)`.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Typed failure for leaf-exec resolution, so callers can match on the
/// specific cause rather than inspecting an error string.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ResolveExecError {
    /// The command id was not found in any source (config, stack, extension).
    #[error(transparent)]
    Unknown(#[from] UnknownCommand),
    /// The command exists but is a composite; leaf plans must be exec-only.
    #[error("internal error: composite in leaf plan: {0}")]
    CompositeInLeafPlan(String),
}

/// Typed failure for composite expansion, so callers can match on the
/// specific cause rather than inspecting an error string.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum ExpandError {
    /// A referenced id was not defined anywhere.
    #[error(transparent)]
    Unknown(#[from] UnknownCommand),
    /// A composite transitively references itself.
    #[error("cycle detected in composite command: {0}")]
    Cycle(String),
    /// Expansion exceeded the safety depth cap.
    #[error("composite expansion exceeded depth limit {max_depth} at command `{id}`")]
    DepthExceeded { id: String, max_depth: usize },
    /// A composite tree declares conflicting values for a scheduling flag
    /// (`parallel` or `fail_fast`).
    ///
    /// Expansion flattens a composite tree into a single flat leaf plan that
    /// the runner schedules as one unit, so exactly one value per flag can be
    /// honoured. Folding conflicting values together instead — letting a
    /// `parallel = true` descendant promote a `parallel = false` ancestor, or
    /// a `fail_fast = false` descendant disable fail-fast plan-wide — would
    /// make the flag mean something other than what it says, with an
    /// intermittent failure mode (formatters racing checkers over the same
    /// files). Rejecting at expansion time makes the trap loud.
    #[error(
        "conflicting `{flag}` in the plan for `{root}`: `{root}` sets {flag} = {root_value}, \
         but `{conflicting}` sets {flag} = {conflicting_value}\n\
         composite commands are flattened into one plan and scheduled as a single unit, \
         so mixed `{flag}` values cannot both be honoured\n\
         fix: make them agree — set `{conflicting}.{flag} = {root_value}`, \
         or set `{root}.{flag} = {conflicting_value}`"
    )]
    ConflictingSchedule {
        /// The scheduling flag that disagrees: `parallel` or `fail_fast`.
        flag: &'static str,
        /// First composite visited in the plan (the expansion root).
        root: String,
        /// The value `root` declared for `flag`.
        root_value: bool,
        /// The composite that disagreed with `root`.
        conflicting: String,
        /// The value `conflicting` declared for `flag`.
        conflicting_value: bool,
    },
}

use exec::exec_command;
use indexmap::IndexMap;
use ops_core::config::{CommandId, CommandSpec, Config, ExecCommandSpec, OutputConfig};
use ops_core::expand::Variables;
use ops_core::stack::Stack;
use ops_extension::{DataProviderError, DataRegistry};
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, instrument};

/// Runs commands from config; emits `RunnerEvent` stream.
pub struct CommandRunner {
    pub(super) config: Arc<Config>,
    // OWN-2 / TASK-0462: Arc-wrapped so the parallel hot path only does
    // Arc::clone (atomic refcount bump) per spawn rather than deep-cloning
    // the inner `PathBuf` / `HashMap`. Sequential callers wrap once at
    // construction.
    pub(super) cwd: Arc<PathBuf>,
    pub(super) vars: Arc<Variables>,
    pub(super) stack_commands: IndexMap<CommandId, CommandSpec>,
    pub(super) extension_commands: IndexMap<CommandId, CommandSpec>,
    /// Always-available commands that mirror clap-level subcommands
    /// (`end-of-file-fixer`, `trailing-whitespace`, …). Registered by
    /// [`builtins::builtin_commands`] so composite `commands = [...]` lists
    /// can reference them by name or visible-alias. Lowest priority in
    /// resolution so user config / stack / extension entries can still
    /// shadow.
    pub(super) builtin_commands: IndexMap<CommandId, CommandSpec>,
    /// Pre-built `alias → canonical` map over the stack + extension command
    /// stores, keeping `canonical_with_spec` / `resolve_alias` O(1) rather
    /// than O(N·A) per lookup. Config aliases are served by
    /// `Config::resolve_alias`, which maintains its own map. Rebuilt when
    /// `register_commands` mutates the extension store.
    pub(super) non_config_alias_map: std::collections::HashMap<String, String>,
    pub(super) data_registry: DataRegistry,
    /// Single source of truth for the per-runner data cache. Storing the
    /// [`ops_extension::Context`] directly means transitive
    /// `ctx.get_or_provide(...)` results survive across `query_data`
    /// calls (a provider that composes others does not pay recompute cost
    /// on every outer query) and `in_flight` markers are not duplicated
    /// state.
    pub(super) data_context: ops_extension::Context,
    pub(super) detected_stack: Option<Stack>,
    /// Cwd-escape policy applied to every spawn this runner orchestrates.
    /// Hook-triggered entry points construct the runner with
    /// `CwdEscapePolicy::Deny` so a coworker-landed `.ops.toml` cannot
    /// escape the workspace on the next commit; the default interactive
    /// path keeps `WarnAndAllow`.
    pub(super) cwd_escape_policy: CwdEscapePolicy,
    /// Bounded, runner-scoped cache of `canonicalize(workspace)` results.
    /// The cache is bounded (LRU eviction at
    /// [`build::WORKSPACE_CANONICAL_CACHE_CAP`]); folding it onto the
    /// runner means its lifetime ends with the runner instead of the
    /// process. The runner exposes [`Self::invalidate_workspace_cache`]
    /// so embedders that observe an on-disk symlink swap can force a
    /// re-canonicalize without dropping the runner.
    ///
    /// This is the authoritative instance: the spawn path threads a clone of
    /// it into `build_command_async` via [`Self::exec_env`], so every escape
    /// decision consults the same cache the invalidate APIs mutate.
    pub(super) workspace_cache: Arc<WorkspaceCanonicalCache>,
}

impl CommandRunner {
    #[must_use]
    pub fn new(config: Config, cwd: PathBuf) -> Self {
        Self::from_arc_config(Arc::new(config), cwd)
    }

    /// Construct a runner from an already-shared `Arc<Config>`.
    ///
    /// Callers that already hold the loaded config behind an `Arc` (the CLI
    /// threads `early_config` from `main` through `dispatch` into here) avoid
    /// a deep clone of the inner `Config` — every nested `IndexMap`,
    /// `String`, and theme block is shared rather than duplicated per CLI
    /// invocation.
    pub fn from_arc_config(config: Arc<Config>, cwd: PathBuf) -> Self {
        let detected_stack = Stack::resolve(config.stack.as_deref(), &cwd);

        let stack_commands: IndexMap<CommandId, CommandSpec> =
            detected_stack.map_or_else(IndexMap::new, |stack| {
                let defaults = stack.default_commands();
                debug!(
                    stack = stack.as_str(),
                    command_count = defaults.len(),
                    "loaded stack default commands"
                );
                defaults
                    .into_iter()
                    .map(|(k, v)| (CommandId::from(k), v))
                    .collect()
            });

        // ERR-1 / TASK-1462: a non-UTF-8 workspace root would otherwise
        // lossy-render into the OPS_ROOT builtin and defeat the
        // strict-expand contract. `Variables::from_env` surfaces it as an
        // `ExpandError` instead; the runner constructor is infallible, so
        // we degrade through a `tracing::warn!` + fail-closed fallback
        // rather than panicking the CLI.
        //
        // SEC-31 / TASK-1854: the fallback is `Variables::poisoned`, not
        // the old empty-builtins `Variables`. The latter was believed to
        // "fail loud on the missing variable", but `shellexpand` leaves an
        // undefined reference literal, so `try_expand("$OPS_ROOT/...")`
        // returned `Ok("$OPS_ROOT/...")` and that literal was materialised
        // as an argv element or cwd — and an ambient `OPS_ROOT` resolved to
        // an unrelated directory rather than failing. A poisoned
        // `Variables` returns the original error from every `try_expand`,
        // so strict callers really do fail. Strict downstream callers
        // should adopt the Result-returning `Variables::from_env` directly.
        let vars = match Variables::from_env(&cwd) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!(
                    var = %e.var_name,
                    cause = %e.cause,
                    "Variables::from_env failed; every $OPS_ROOT expansion will surface this error"
                );
                Variables::poisoned(e)
            }
        };
        let extension_commands = IndexMap::new();
        let builtin_commands = builtins::builtin_commands();
        let non_config_alias_map = resolve::build_alias_map(
            std::iter::once(&stack_commands)
                .chain(std::iter::once(&extension_commands))
                .chain(std::iter::once(&builtin_commands)),
        );

        let cwd = Arc::new(cwd);
        let data_context =
            ops_extension::Context::from_cwd_arc(Arc::clone(&config), Arc::clone(&cwd));

        Self {
            config,
            cwd,
            vars: Arc::new(vars),
            stack_commands,
            extension_commands,
            builtin_commands,
            non_config_alias_map,
            data_registry: DataRegistry::new(),
            data_context,
            detected_stack,
            cwd_escape_policy: CwdEscapePolicy::WarnAndAllow,
            workspace_cache: Arc::new(WorkspaceCanonicalCache::new()),
        }
    }

    /// Forget the cached canonicalization for `workspace`.
    ///
    /// The next escape check re-runs `std::fs::canonicalize`, picking up any
    /// symlink swap that happened after the entry was cached.
    pub fn invalidate_workspace_cache(&self, workspace: &std::path::Path) {
        self.workspace_cache.invalidate(workspace);
    }

    /// Drop every cached workspace canonicalization, for embedders that know
    /// the layout has changed wholesale.
    pub fn clear_workspace_cache(&self) {
        self.workspace_cache.clear();
    }

    /// Set the cwd-escape policy for every spawn this runner orchestrates.
    ///
    /// Hook-triggered entry points (`run-before-commit`, `run-before-push`)
    /// pass [`CwdEscapePolicy::Deny`] so a `.ops.toml` `cwd = "/etc"` or
    /// `cwd = "../../"` is refused at spawn time instead of producing a
    /// tracing warning and proceeding.
    pub const fn set_cwd_escape_policy(&mut self, policy: CwdEscapePolicy) {
        self.cwd_escape_policy = policy;
    }

    /// Merge a single `(id, spec)` pair into the non-config alias map
    /// without re-iterating the stack + extension stores.
    ///
    /// The incremental merge is O(aliases-of-spec) per registration, so N
    /// successive single-entry `register_commands` calls stay linear rather
    /// than O(N · (|stack| + |extensions|)). Stale aliases owned by an
    /// earlier version of the same id are pruned first, so a re-registration
    /// that drops an alias does not leave the map pointing at a spec that no
    /// longer claims it.
    fn merge_alias_for(&mut self, id: &CommandId, new_spec: &CommandSpec) {
        // Both branches route through the `Entry` API so each alias is
        // looked up exactly once; a `get` → `remove` / `get` → `insert` pair
        // would probe the map twice and invite drift between the lookups.
        use std::collections::hash_map::Entry;
        if let Some(old_spec) = self.extension_commands.get(id) {
            for old_alias in old_spec.aliases() {
                if let Entry::Occupied(occ) = self
                    .non_config_alias_map
                    .entry(old_alias.as_str().to_string())
                {
                    if occ.get() == id.as_str() {
                        occ.remove();
                    }
                }
            }
        }
        for alias in new_spec.aliases() {
            // CONC-3 / TASK-1137: also flag cross-store collisions against
            // the config alias map. `register_commands` already warns on
            // duplicate command-id registration (SEC-31 / TASK-0402) and
            // intra-store alias collisions are caught below; without this
            // check, an extension whose alias matches a config-defined
            // alias is silently shadowed at lookup time (config wins via
            // `resolve_alias` ordering at resolve.rs) with no audit trail
            // for operators reading `RUST_LOG=ops=debug`.
            if let Some(config_owner) = self.config.resolve_alias(alias.as_str()) {
                if config_owner != id.as_str() {
                    tracing::warn!(
                        alias = ?alias,
                        config_owner = ?config_owner,
                        new = ?id.as_str(),
                        "alias collision: extension/stack alias shadowed by config alias of same name"
                    );
                }
            }
            match self.non_config_alias_map.entry(alias.clone()) {
                Entry::Occupied(mut occ) => {
                    if occ.get() != id.as_str() {
                        tracing::warn!(
                            alias = ?alias,
                            existing = ?occ.get(),
                            new = ?id.as_str(),
                            "alias collision: later store overrides earlier"
                        );
                    }
                    occ.insert(id.to_string());
                }
                Entry::Vacant(vac) => {
                    vac.insert(id.to_string());
                }
            }
        }
    }

    /// Full config (for extensions that need data path, etc.).
    #[must_use]
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Working directory (e.g. for resolving DB path).
    #[must_use]
    pub fn working_directory(&self) -> &std::path::Path {
        &self.cwd
    }

    /// Output/theme config for formatting step lines.
    #[must_use]
    pub fn output_config(&self) -> &OutputConfig {
        &self.config.output
    }

    /// Variable expansion context for command specs.
    #[must_use]
    pub fn variables(&self) -> &Variables {
        &self.vars
    }

    /// Detected or configured stack.
    #[must_use]
    pub const fn stack(&self) -> Option<Stack> {
        self.detected_stack
    }

    /// Replace the internal data registry (e.g. with one populated by extensions).
    ///
    /// Also drops every entry the runner's `data_context` cached against the
    /// outgoing registry. Without that invalidation, a later
    /// [`Self::query_data`] for a key whose provider was replaced or removed
    /// would still hand back a stale `Arc<serde_json::Value>`. Calling this
    /// is the operator's signal to rebuild the data view, and the cache
    /// follows it.
    pub fn register_data_providers(&mut self, registry: DataRegistry) {
        self.data_registry = registry;
        self.data_context.clear_provider_results();
    }

    /// Query cached data or compute via provider.
    ///
    /// Dispatches into the persistent [`ops_extension::Context`] held on the
    /// runner, so transitive `ctx.get_or_provide(other)` calls made inside a
    /// provider are cached too: composed providers pay the inner cost once
    /// per runner rather than once per query.
    ///
    /// # Errors
    ///
    /// Whatever the named provider returns; see [`ops_extension::Context::get_or_provide`].
    pub fn query_data(&mut self, name: &str) -> Result<Arc<serde_json::Value>, DataProviderError> {
        self.data_context.get_or_provide(name, &self.data_registry)
    }

    /// Register commands from extensions (merged with config commands).
    ///
    /// Duplicates are detected at this final consolidation point: an id
    /// already present in the extension store logs a warning, so the CLI
    /// shadowing behaviour is never silent. (Duplicates seen earlier, in
    /// `register_extension_commands`, have already warned there.)
    pub fn register_commands(
        &mut self,
        commands: impl IntoIterator<Item = (CommandId, CommandSpec)>,
    ) {
        for (id, spec) in commands {
            if self.extension_commands.contains_key(&id) {
                tracing::warn!(
                    command = ?id.as_str(),
                    "duplicate extension command registration; later registration shadows earlier"
                );
            }
            // Merge this entry's aliases into the alias map before swapping
            // the spec into the store, so the outgoing spec (if any) is still
            // visible and its aliases can be pruned.
            self.merge_alias_for(&id, &spec);
            self.extension_commands.insert(id, spec);
        }
    }

    /// Bundle the runner-scoped execution handles for the spawn paths this
    /// runner owns (`run_exec`, `run_plan_raw`, `spawn_parallel_tasks`).
    /// Building one costs an `Arc` refcount bump per field.
    fn exec_env(&self) -> exec::ExecEnv {
        exec::ExecEnv {
            cwd: Arc::clone(&self.cwd),
            vars: Arc::clone(&self.vars),
            policy: self.cwd_escape_policy,
            workspace_cache: Arc::clone(&self.workspace_cache),
        }
    }

    /// Run a single exec command; returns result and can stream output via callback.
    #[instrument(skip(self, on_event), fields(id = ?id))]
    pub async fn run_exec(
        &self,
        id: &str,
        spec: &std::sync::Arc<ExecCommandSpec>,
        on_event: &mut impl FnMut(RunnerEvent),
    ) -> StepResult {
        // PERF-3 / TASK-1125: `&Arc<ExecCommandSpec>` — Arc::clone per
        // build_command_async dispatch, no spec deep clone per spawn. The
        // `ExecEnv` handles Arc::clone once each if the build needs to
        // spawn_blocking, no deep clone.
        exec_command(id, spec, &self.exec_env(), on_event).await
    }

    /// Run a named command (single or composite); returns step results.
    ///
    /// # Errors
    ///
    /// If `command_id` resolves to nothing, if composite expansion fails
    /// (unknown reference, cycle, depth limit, conflicting schedule flags), or
    /// if a step cannot be built or spawned.
    // Same `!Send` reasoning as `run_plan_parallel`: the `on_event` sink is
    // backed by non-`Send` `indicatif` state (docs/clippy.md layer 3).
    #[allow(clippy::future_not_send)]
    pub async fn run(
        &self,
        command_id: &str,
        on_event: &mut impl FnMut(RunnerEvent),
    ) -> anyhow::Result<Vec<StepResult>> {
        // PERF-3 / TASK-2086: one walk of the command stores. The plan and
        // the scheduling flags both come from `expand_to_leaves_with_flags`
        // (PATTERN-1 / TASK-1283); a separate `resolve` of the root was a
        // second traversal and a second source of truth for the same
        // decision. The aggregated flags are equivalent to the root spec's
        // own: TASK-1657's agreement check errors on any tree that declares
        // conflicting values, and an Exec root contributes
        // `(any_parallel=false, fail_fast_disabled=false)` — the single
        // fail-fast sequential step it always ran as.
        let (plan, any_parallel, fail_fast_disabled) = self
            .expand_to_leaves_with_flags(command_id)
            .map_err(anyhow::Error::from)?;
        debug!(command_id, steps = plan.len(), "running command");

        let fail_fast = !fail_fast_disabled;
        let results = if any_parallel {
            self.run_plan_parallel(&plan, fail_fast, on_event).await
        } else {
            self.run_plan(&plan, fail_fast, on_event).await
        };
        Ok(results)
    }
}

#[cfg(test)]
mod tests;
