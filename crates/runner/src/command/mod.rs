//! Command execution engine: exec and composite commands, RunnerEvent stream.
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
mod events;
mod exec;
mod parallel;
mod resolve;
mod results;
mod secret_patterns;
mod sequential;

pub use build::CwdEscapePolicy;
pub use events::{OutputLine, RunnerEvent};
pub use results::StepResult;
pub use secret_patterns::is_sensitive_env_key;
pub use secret_patterns::looks_like_secret_value as looks_like_secret_value_public;

/// Shared "id not found in any store" failure. DUP-3 / TASK-0769:
/// [`ResolveExecError`] and [`ExpandError`] previously each defined an
/// `Unknown(String)` variant with identical Display strings. Both now wrap
/// this single struct so the message lives in one place and a future
/// caller can convert between the parent enums via `#[from]` without
/// reconstructing the inner string.
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

/// Typed failure for leaf-exec resolution. ERR-10 / TASK-0130: replaces
/// stringly-typed errors so callers can match on the specific cause.
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

/// Typed failure for composite expansion. ERR-10 / READ-5 / TASK-0203+0215.
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

/// Runs commands from config; emits RunnerEvent stream.
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
    /// OWN-6 / TASK-0200: pre-built `alias → canonical` map over the
    /// stack + extension command stores so `canonical_id` / `resolve_alias`
    /// are O(1) instead of O(N·A) per lookup. Config aliases are served by
    /// `Config::resolve_alias` which maintains its own map. Rebuilt when
    /// `register_commands` mutates the extension store.
    pub(super) non_config_alias_map: std::collections::HashMap<String, String>,
    pub(super) data_registry: DataRegistry,
    pub(super) data_cache: std::collections::HashMap<String, Arc<serde_json::Value>>,
    pub(super) detected_stack: Option<Stack>,
    /// SEC-14 / TASK-0886: cwd-escape policy applied to every spawn this
    /// runner orchestrates. Hook-triggered entry points construct the
    /// runner with `CwdEscapePolicy::Deny` so a coworker-landed `.ops.toml`
    /// cannot escape the workspace on the next commit; the default
    /// interactive path keeps `WarnAndAllow`.
    pub(super) cwd_escape_policy: CwdEscapePolicy,
}

impl CommandRunner {
    pub fn new(config: Config, cwd: PathBuf) -> Self {
        Self::from_arc_config(Arc::new(config), cwd)
    }

    /// OWN-2 / TASK-0841: construct a runner directly from an already-shared
    /// `Arc<Config>`. Callers that already hold the loaded config behind an
    /// `Arc` (the CLI threads `early_config` from `main` through `dispatch`
    /// into here) avoid the deep clone of the inner `Config` — every nested
    /// `IndexMap`, `String`, and theme block is shared rather than duplicated
    /// per CLI invocation.
    pub fn from_arc_config(config: Arc<Config>, cwd: PathBuf) -> Self {
        let detected_stack = Stack::resolve(config.stack.as_deref(), &cwd);

        let stack_commands: IndexMap<CommandId, CommandSpec> = if let Some(stack) = detected_stack {
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
        } else {
            IndexMap::new()
        };

        let vars = Variables::from_env(&cwd);
        let extension_commands = IndexMap::new();
        let non_config_alias_map = resolve::build_alias_map(
            std::iter::once(&stack_commands).chain(std::iter::once(&extension_commands)),
        );

        Self {
            config,
            cwd: Arc::new(cwd),
            vars: Arc::new(vars),
            stack_commands,
            extension_commands,
            non_config_alias_map,
            data_registry: DataRegistry::new(),
            data_cache: std::collections::HashMap::new(),
            detected_stack,
            cwd_escape_policy: CwdEscapePolicy::WarnAndAllow,
        }
    }

    /// SEC-14 / TASK-0886: opt this runner into the fail-closed cwd-escape
    /// policy. Hook-triggered entry points (`run-before-commit`,
    /// `run-before-push`) call this with `CwdEscapePolicy::Deny` so a
    /// `.ops.toml` `cwd = "/etc"` or `cwd = "../../"` is refused at spawn
    /// time instead of producing a tracing warning and proceeding.
    pub fn set_cwd_escape_policy(&mut self, policy: CwdEscapePolicy) {
        self.cwd_escape_policy = policy;
    }

    /// PERF-3 / TASK-0774: merge a single (id, spec) pair into the
    /// non-config alias map without re-iterating the stack + extension
    /// stores. Earlier the registration path called `build_alias_map` over
    /// every store on each batch, which made N successive
    /// `register_commands` calls of one entry each O(N · (|stack| +
    /// |extensions|)). Incremental merge keeps that work O(aliases-of-spec)
    /// per registration. Stale aliases owned by an earlier version of the
    /// same id are pruned first so a re-registration that drops an alias
    /// does not leave the map pointing at a now-invalid spec.
    fn merge_alias_for(&mut self, id: &CommandId, new_spec: &CommandSpec) {
        // PATTERN-1 / TASK-0998: route both branches through the `Entry`
        // API so each alias is looked up exactly once. The previous
        // `get` → `remove` and `get` → `insert` pairs probed the map
        // twice and invited drift between the two lookups.
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
            match self.non_config_alias_map.entry(alias.clone()) {
                Entry::Occupied(mut occ) => {
                    if occ.get() != id.as_str() {
                        tracing::warn!(
                            alias = %alias,
                            existing = %occ.get(),
                            new = %id,
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
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Working directory (e.g. for resolving DB path).
    pub fn working_directory(&self) -> &std::path::Path {
        &self.cwd
    }

    /// Output/theme config for formatting step lines.
    pub fn output_config(&self) -> &OutputConfig {
        &self.config.output
    }

    /// Variable expansion context for command specs.
    pub fn variables(&self) -> &Variables {
        &self.vars
    }

    /// Detected or configured stack.
    pub fn stack(&self) -> Option<Stack> {
        self.detected_stack
    }

    /// Replace the internal data registry (e.g. with one populated by extensions).
    pub fn register_data_providers(&mut self, registry: DataRegistry) {
        self.data_registry = registry;
    }

    /// Query cached data or compute via provider.
    pub fn query_data(&mut self, name: &str) -> Result<Arc<serde_json::Value>, DataProviderError> {
        use std::collections::hash_map::Entry;
        match self.data_cache.entry(name.to_string()) {
            Entry::Occupied(v) => Ok(Arc::clone(v.get())),
            Entry::Vacant(slot) => {
                let mut ctx = ops_extension::Context::from_cwd_arc(
                    Arc::clone(&self.config),
                    Arc::clone(&self.cwd),
                );
                let v = ctx.get_or_provide(name, &self.data_registry)?;
                slot.insert(Arc::clone(&v));
                Ok(v)
            }
        }
    }

    /// Register commands from extensions (merged with config commands).
    ///
    /// SEC-31 / TASK-0402: detect duplicates at this final consolidation
    /// point. If two extensions registered the same id under
    /// `register_extension_commands`, the upstream warning already fired;
    /// here we emit a warning if a same id appears more than once in this
    /// call (e.g. multiple `register_commands` invocations) so the CLI
    /// shadowing behaviour is never silent.
    pub fn register_commands(
        &mut self,
        commands: impl IntoIterator<Item = (CommandId, CommandSpec)>,
    ) {
        for (id, spec) in commands {
            if self.extension_commands.contains_key(&id) {
                tracing::warn!(
                    command = %id,
                    "duplicate extension command registration; later registration shadows earlier"
                );
            }
            // PERF-3 / TASK-0774: merge this entry's aliases into the alias
            // map before swapping the spec into the store, so we still see
            // the previous spec (if any) and can prune its aliases.
            self.merge_alias_for(&id, &spec);
            self.extension_commands.insert(id, spec);
        }
    }

    /// Run a single exec command; returns result and can stream output via callback.
    #[instrument(skip(self, on_event), fields(id = %id))]
    pub async fn run_exec(
        &self,
        id: &str,
        spec: &ExecCommandSpec,
        on_event: &mut impl FnMut(RunnerEvent),
    ) -> StepResult {
        exec_command(
            id,
            spec,
            &self.cwd,
            &self.vars,
            self.cwd_escape_policy,
            on_event,
        )
        .await
        // ↑ `&Arc<PathBuf>` / `&Arc<Variables>` — exec_command Arc::clones
        // once if the build needs to spawn_blocking, no deep clone.
    }

    /// Run a named command (single or composite); returns step results.
    pub async fn run(
        &self,
        command_id: &str,
        on_event: &mut impl FnMut(RunnerEvent),
    ) -> anyhow::Result<Vec<StepResult>> {
        let spec = self
            .resolve(command_id)
            .ok_or_else(|| ExpandError::Unknown(UnknownCommand::new(command_id)))?;
        let plan = self
            .expand_to_leaves(command_id)
            .map_err(anyhow::Error::from)?;
        debug!(command_id, steps = plan.len(), "running command");

        let results = match spec {
            CommandSpec::Composite(c) if c.parallel => {
                self.run_plan_parallel(&plan, c.fail_fast, on_event).await
            }
            CommandSpec::Composite(c) => self.run_plan(&plan, c.fail_fast, on_event).await,
            _ => self.run_plan(&plan, true, on_event).await,
        };
        Ok(results)
    }
}

#[cfg(test)]
mod tests;
