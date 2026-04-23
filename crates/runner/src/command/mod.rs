//! Command execution engine: exec and composite commands, RunnerEvent stream.
//!
//! # Architecture (CQ-008)
//!
//! `CommandRunner` is the central orchestrator for command execution. It handles:
//! - **Command resolution**: Looking up commands from config, stack defaults, or extensions
//! - **Command expansion**: Flattening composite commands into exec leaves
//! - **Sequential execution**: Running commands one after another
//! - **Parallel execution**: Running commands concurrently with fail-fast support
//! - **Data caching**: Memoizing provider results
//!
//! ## Command Resolution Priority
//!
//! Commands are resolved in this order (highest to lowest priority):
//! 1. **Config commands**: From `.ops.toml` or internal defaults
//! 2. **Stack commands**: Language/stack-specific defaults (e.g., `cargo fmt` for Rust)
//! 3. **Extension commands**: Commands registered by extensions
//!
//! ## Future Refactoring
//!
//! If this struct continues to grow, consider splitting into:
//! - `CommandResolver`: Config + extension command lookup
//! - `CommandExpander`: Composite → exec leaf expansion
//! - `CommandExecutor`: Sequential/parallel execution with event emission
//!
//! Currently kept as a single struct because:
//! 1. All concerns share the same config and cwd context
//! 2. Data caching needs to span resolution and execution
//! 3. The public API is stable and well-tested

mod events;
mod exec;
mod results;

pub use events::RunnerEvent;
pub use exec::is_sensitive_env_key;
pub use exec::looks_like_secret_value as looks_like_secret_value_public;
pub use results::StepResult;

/// Typed failure for leaf-exec resolution. ERR-10 / TASK-0130: replaces
/// stringly-typed errors so callers can match on the specific cause.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ResolveExecError {
    /// The command id was not found in any source (config, stack, extension).
    #[error("unknown command: {0}")]
    Unknown(String),
    /// The command exists but is a composite; leaf plans must be exec-only.
    #[error("internal error: composite in leaf plan: {0}")]
    CompositeInLeafPlan(String),
}

/// Typed failure for composite expansion. ERR-10 / READ-5 / TASK-0203+0215:
/// `expand_to_leaves` previously returned `Option<Vec<CommandId>>`, which
/// conflated three distinct failure modes into one `None` that callers
/// universally rendered as "unknown command". This enum lets the CLI
/// surface the actual cause.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ExpandError {
    /// A referenced id was not defined anywhere.
    #[error("unknown command: {0}")]
    Unknown(String),
    /// A composite transitively references itself.
    #[error("cycle detected in composite command: {0}")]
    Cycle(String),
    /// Expansion exceeded the safety depth cap.
    #[error("composite expansion exceeded depth limit {max_depth} at command `{id}`")]
    DepthExceeded { id: String, max_depth: usize },
}

use exec::{exec_command, exec_command_raw, exec_standalone, resolution_failure};
use indexmap::IndexMap;
use ops_core::config::{CommandId, CommandSpec, Config, ExecCommandSpec, OutputConfig};
use ops_core::expand::Variables;
use ops_core::stack::Stack;
use ops_extension::{DataProviderError, DataRegistry};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, instrument};

/// Tracks the lifecycle of a plan execution (PlanStarted → RunFinished bookends).
struct PlanLifecycle {
    start: Instant,
}

impl PlanLifecycle {
    fn begin(command_ids: &[CommandId], on_event: &mut impl FnMut(RunnerEvent)) -> Self {
        on_event(RunnerEvent::PlanStarted {
            command_ids: command_ids.to_vec(),
        });
        Self {
            start: Instant::now(),
        }
    }

    /// FN-9 / TASK-0197+0211: take `success` explicitly rather than a full
    /// `&[StepResult]`. Callers already walk the results inside the run loop
    /// to compute success anyway, so threading a bool is clearer than
    /// handing over the entire slice for an `iter().all()` re-walk. It also
    /// prevents a future refactor from passing a partial-result slice and
    /// silently misreporting the run outcome.
    fn finish(self, success: bool, on_event: &mut impl FnMut(RunnerEvent)) {
        on_event(RunnerEvent::RunFinished {
            duration_secs: self.start.elapsed().as_secs_f64(),
            success,
        });
    }
}

/// Runs commands from config; emits RunnerEvent stream.
pub struct CommandRunner {
    config: Arc<Config>,
    cwd: PathBuf,
    vars: Variables,
    stack_commands: IndexMap<CommandId, CommandSpec>,
    extension_commands: IndexMap<CommandId, CommandSpec>,
    /// OWN-6 / TASK-0200: pre-built `alias → canonical` map over the
    /// stack + extension command stores so `canonical_id` / `resolve_alias`
    /// are O(1) instead of O(N·A) per lookup. Config aliases are served by
    /// `Config::resolve_alias` which maintains its own map. Rebuilt when
    /// `register_commands` mutates the extension store.
    non_config_alias_map: std::collections::HashMap<String, String>,
    data_registry: DataRegistry,
    data_cache: std::collections::HashMap<String, Arc<serde_json::Value>>,
    detected_stack: Option<Stack>,
}

impl CommandRunner {
    pub fn new(config: Config, cwd: PathBuf) -> Self {
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
        let non_config_alias_map = build_alias_map(
            std::iter::once(&stack_commands).chain(std::iter::once(&extension_commands)),
        );

        Self {
            config: Arc::new(config),
            cwd,
            vars,
            stack_commands,
            extension_commands,
            non_config_alias_map,
            data_registry: DataRegistry::new(),
            data_cache: std::collections::HashMap::new(),
            detected_stack,
        }
    }

    fn rebuild_alias_map(&mut self) {
        self.non_config_alias_map = build_alias_map(
            std::iter::once(&self.stack_commands).chain(std::iter::once(&self.extension_commands)),
        );
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
                let mut ctx =
                    ops_extension::Context::new(Arc::clone(&self.config), self.cwd.clone());
                let v = ctx.get_or_provide(name, &self.data_registry)?;
                slot.insert(Arc::clone(&v));
                Ok(v)
            }
        }
    }

    /// Register commands from extensions (merged with config commands).
    pub fn register_commands(
        &mut self,
        commands: impl IntoIterator<Item = (CommandId, CommandSpec)>,
    ) {
        for (id, spec) in commands {
            self.extension_commands.insert(id, spec);
        }
        self.rebuild_alias_map();
    }

    /// Iterator over all command keys across config → stack → extension.
    fn all_command_keys(&self) -> impl Iterator<Item = &str> {
        self.config
            .commands
            .keys()
            .map(|s| s.as_str())
            .chain(self.stack_commands.keys().map(|k| k.as_str()))
            .chain(self.extension_commands.keys().map(|k| k.as_str()))
    }

    /// Look up a command by ID across all stores (config → stack → extension).
    fn find_in_stores(&self, id: &str) -> Option<&CommandSpec> {
        self.config
            .commands
            .get(id)
            .or_else(|| self.stack_commands.get(id))
            .or_else(|| self.extension_commands.get(id))
    }

    /// Check if a command ID exists in any store.
    fn exists_in_stores(&self, id: &str) -> bool {
        self.config.commands.contains_key(id)
            || self.stack_commands.contains_key(id)
            || self.extension_commands.contains_key(id)
    }

    /// Resolve a command by ID or alias (config first, then stack defaults, then extension, then aliases).
    pub fn resolve(&self, id: &str) -> Option<&CommandSpec> {
        self.find_in_stores(id).or_else(|| self.resolve_alias(id))
    }

    /// Return the canonical command name for a given ID or alias.
    /// If the ID is already a direct command name, returns it as-is.
    /// If it matches an alias, returns the canonical name.
    ///
    /// OWN-6 / TASK-0200: alias search is O(1) via `non_config_alias_map`
    /// instead of scanning every spec's aliases list.
    fn canonical_id<'a>(&'a self, id: &'a str) -> &'a str {
        if self.exists_in_stores(id) {
            return id;
        }
        if let Some(name) = self.config.resolve_alias(id) {
            return name;
        }
        if let Some(name) = self.non_config_alias_map.get(id) {
            return name.as_str();
        }
        id
    }

    /// Look up a command by alias across all command sources.
    fn resolve_alias(&self, alias: &str) -> Option<&CommandSpec> {
        // Config aliases use a dedicated method (separate alias map)
        if let Some(name) = self.config.resolve_alias(alias) {
            return self.config.commands.get(name);
        }
        let canonical = self.non_config_alias_map.get(alias)?;
        self.stack_commands
            .get(canonical.as_str())
            .or_else(|| self.extension_commands.get(canonical.as_str()))
    }

    /// List all available command IDs (config first, then stack, then extension commands; sorted for stable order).
    pub fn list_command_ids(&self) -> Vec<CommandId> {
        let mut ids: Vec<&str> = self.all_command_keys().collect();
        ids.sort_unstable();
        ids.dedup();
        ids.iter().map(|s| CommandId::from(*s)).collect()
    }

    /// Expand to a flat list of exec-only command IDs (no composites), so run_plan need not recurse.
    ///
    /// Returns [`ExpandError`] distinguishing the three distinct failure modes
    /// — unknown id, cycle, depth exceeded — so callers can render accurate
    /// diagnostics instead of blanket "unknown command". (ERR-10 / READ-5.)
    ///
    /// # Recursion Depth
    ///
    /// The recursion is bounded by the cycle detection mechanism - each composite can only
    /// be visited once per expansion. For deeply nested composites, the call stack depth is
    /// limited by the number of unique composites, not the total depth. In practice, this
    /// means a graph with N composites has at most N stack frames during expansion.
    ///
    /// An additional guard limits expansion to 100 levels to prevent pathological cases.
    pub fn expand_to_leaves(&self, id: &str) -> Result<Vec<CommandId>, ExpandError> {
        /// CQ-012: Maximum recursion depth for composite expansion.
        ///
        /// This limit prevents stack overflow from pathological configs with deeply
        /// nested composites (e.g., a -> b -> c -> ... -> z with 100+ levels). Normal
        /// configs typically have 2-5 levels (e.g., verify -> [build, test] -> cargo).
        /// The cycle detection already catches circular references, so this is a
        /// defense against accidental deep nesting.
        const MAX_DEPTH: usize = 100;
        let mut visited = std::collections::HashSet::new();
        self.expand_inner(id, &mut visited, 0, MAX_DEPTH)
    }

    fn expand_inner(
        &self,
        id: &str,
        visited: &mut std::collections::HashSet<String>,
        depth: usize,
        max_depth: usize,
    ) -> Result<Vec<CommandId>, ExpandError> {
        if depth > max_depth {
            tracing::warn!(
                id = %id,
                depth = depth,
                max_depth = max_depth,
                "composite expansion depth limit exceeded"
            );
            return Err(ExpandError::DepthExceeded {
                id: id.to_string(),
                max_depth,
            });
        }
        let canonical = self.canonical_id(id);
        let spec = self
            .resolve(canonical)
            .ok_or_else(|| ExpandError::Unknown(id.to_string()))?;
        match spec {
            CommandSpec::Exec(_) => Ok(vec![CommandId::from(canonical)]),
            CommandSpec::Composite(c) => {
                if !visited.insert(canonical.to_string()) {
                    return Err(ExpandError::Cycle(canonical.to_string()));
                }
                let mut out = Vec::new();
                for sub in &c.commands {
                    out.extend(self.expand_inner(sub, visited, depth + 1, max_depth)?);
                }
                Ok(out)
            }
        }
    }

    /// Resolve a leaf ID to an owned [`ExecCommandSpec`], producing a typed
    /// [`ResolveExecError`] that sequential (`execute_step`) and raw
    /// (`run_plan_raw`) paths both surface identically. (ERR-10 / TASK-0130.)
    fn resolve_exec_leaf(&self, id: &str) -> Result<ExecCommandSpec, ResolveExecError> {
        match self.resolve(id) {
            Some(CommandSpec::Exec(e)) => Ok(e.clone()),
            Some(CommandSpec::Composite(_)) => {
                Err(ResolveExecError::CompositeInLeafPlan(id.to_string()))
            }
            None => Err(ResolveExecError::Unknown(id.to_string())),
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
        exec_command(id, spec, &self.cwd, &self.vars, on_event).await
    }

    /// Execute a single step in a sequential plan, returning the result and whether to stop.
    async fn execute_step(
        &self,
        id: &str,
        on_event: &mut impl FnMut(RunnerEvent),
    ) -> (StepResult, bool) {
        match self.resolve_exec_leaf(id) {
            Ok(e) => {
                let r = self.run_exec(id, &e, on_event).await;
                let should_stop = !r.success;
                (r, should_stop)
            }
            Err(err) => (resolution_failure(id, err.to_string(), on_event), true),
        }
    }

    /// Run a flat list of exec command IDs sequentially.
    /// When `fail_fast` is true, stop on first failure.
    #[instrument(skip(self, on_event))]
    pub async fn run_plan(
        &self,
        command_ids: &[CommandId],
        fail_fast: bool,
        on_event: &mut impl FnMut(RunnerEvent),
    ) -> Vec<StepResult> {
        let lifecycle = PlanLifecycle::begin(command_ids, on_event);
        let mut results = Vec::new();

        for id in command_ids {
            let (result, should_stop) = self.execute_step(id, on_event).await;
            results.push(result);
            if fail_fast && should_stop {
                break;
            }
        }

        lifecycle.finish(results.iter().all(|r| r.success), on_event);
        results
    }

    /// Run a flat list of exec command IDs sequentially with inherited stdio (raw mode).
    ///
    /// No `RunnerEvent`s are emitted and no `on_event` callback is accepted —
    /// the child processes write directly to the terminal. Composites are
    /// always run sequentially in raw mode; callers are expected to have
    /// already expanded any composite `parallel` flag away.
    #[instrument(skip(self))]
    pub async fn run_plan_raw(
        &self,
        command_ids: &[CommandId],
        fail_fast: bool,
    ) -> Vec<StepResult> {
        let mut results = Vec::new();
        for id in command_ids {
            let spec = match self.resolve_exec_leaf(id.as_str()) {
                Ok(spec) => spec,
                Err(err) => {
                    results.push(StepResult::failure(
                        id.as_str(),
                        Duration::ZERO,
                        err.to_string(),
                    ));
                    if fail_fast {
                        break;
                    }
                    continue;
                }
            };
            let result = exec_command_raw(id.as_str(), &spec, &self.cwd, &self.vars).await;
            let should_stop = !result.success;
            results.push(result);
            if fail_fast && should_stop {
                break;
            }
        }
        results
    }

    /// Run a named command (single or composite) with inherited stdio (raw mode).
    ///
    /// Mirrors [`CommandRunner::run`] but always sequential and without events.
    pub async fn run_raw(&self, command_id: &str) -> anyhow::Result<Vec<StepResult>> {
        let plan = self
            .expand_to_leaves(command_id)
            .map_err(anyhow::Error::from)?;
        let fail_fast = match self.resolve(command_id) {
            Some(CommandSpec::Composite(c)) => c.fail_fast,
            _ => true,
        };
        debug!(command_id, steps = plan.len(), "running command (raw)");
        Ok(self.run_plan_raw(&plan, fail_fast).await)
    }

    /// Resolve command IDs to exec specs, returning Err with the offending ID on failure.
    fn resolve_exec_specs(
        &self,
        command_ids: &[CommandId],
    ) -> Result<Vec<(CommandId, ExecCommandSpec)>, CommandId> {
        let mut steps = Vec::with_capacity(command_ids.len());
        for id in command_ids {
            match self.resolve(id) {
                // Clone is required: specs must be owned to move into spawned tasks.
                // Acceptable for typical parallel groups (<10 commands).
                Some(CommandSpec::Exec(e)) => steps.push((id.clone(), e.clone())),
                _ => return Err(id.clone()),
            }
        }
        Ok(steps)
    }

    /// Collect results from a JoinSet, handling panics gracefully.
    ///
    /// # Panic Safety
    ///
    /// If a parallel task panics, this method catches the panic via `JoinHandle`
    /// and converts it to a `StepResult::failure` with a descriptive message.
    /// This ensures that:
    /// - The main process does not crash
    /// - All task results are accounted for
    /// - The user sees which task failed
    ///
    /// This is important for robustness in CI/CD environments where a single
    /// misbehaving command should not abort the entire run.
    pub(crate) async fn collect_join_results(
        mut join_set: tokio::task::JoinSet<StepResult>,
    ) -> Vec<StepResult> {
        let mut results = Vec::new();
        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(step_result) => results.push(step_result),
                Err(e) => {
                    // CONC-6 / TASK-0214: distinguish a cancellation
                    // (fail_fast aborted the JoinSet) from a real panic so
                    // users see "cancelled" rather than misleading
                    // "panicked" for siblings that were intentionally
                    // stopped.
                    if e.is_cancelled() {
                        tracing::debug!("parallel task cancelled (fail_fast abort)");
                        results.push(StepResult::skipped(CommandId::from("<cancelled>")));
                    } else {
                        tracing::error!("parallel task panicked: {}", e);
                        results.push(StepResult::failure(
                            "<panicked>",
                            Duration::ZERO,
                            format!("task panicked: {}", e),
                        ));
                    }
                }
            }
        }
        results
    }

    /// Maximum concurrent parallel tasks. Caps resource usage (file descriptors,
    /// processes) for configs with many parallel commands.
    const MAX_PARALLEL: usize = 32;

    /// Spawn parallel tasks into a JoinSet, returning the receiver and abort flag.
    ///
    /// Concurrency is capped at `MAX_PARALLEL` via a semaphore to prevent
    /// resource exhaustion with large parallel groups.
    /// Per-parallel-task event budget used to size the bounded event channel.
    ///
    /// Budget = StepStarted + N×StepOutput + (StepFinished | StepFailed |
    /// StepSkipped). Real commands rarely hit N=256 between display pumps;
    /// when a burst does fill the channel the producer task awaits on
    /// `send`, which naturally back-pressures chatty children instead of
    /// letting the process drift toward OOM.
    const PARALLEL_EVENT_BUDGET_PER_TASK: usize = 256;

    pub(crate) fn spawn_parallel_tasks(
        steps: Vec<(CommandId, ExecCommandSpec)>,
        cwd: PathBuf,
        vars: Variables,
    ) -> (
        mpsc::Receiver<RunnerEvent>,
        Arc<AtomicBool>,
        tokio::task::JoinSet<StepResult>,
    ) {
        // CONC-3 / TASK-0158+0209: bounded channel so a chatty child
        // back-pressures on the display pump instead of growing the mpsc
        // buffer until the process OOMs. Capacity is sized to
        // MAX_PARALLEL × per-task event budget so the steady-state batch
        // of events never blocks; only pathological bursts of >N lines
        // per tick will pause a producer — which is exactly the
        // throttling we want.
        let capacity = Self::MAX_PARALLEL.saturating_mul(Self::PARALLEL_EVENT_BUDGET_PER_TASK);
        let (tx, rx) = mpsc::channel(capacity);
        let abort = Arc::new(AtomicBool::new(false));
        let semaphore = Arc::new(tokio::sync::Semaphore::new(Self::MAX_PARALLEL));
        let cwd = Arc::new(cwd);
        let vars = Arc::new(vars);
        let mut join_set = tokio::task::JoinSet::new();
        for (id, spec) in steps {
            let tx = tx.clone();
            let abort = Arc::clone(&abort);
            let cwd = Arc::clone(&cwd);
            let vars = Arc::clone(&vars);
            let sem = Arc::clone(&semaphore);
            let task_id = id.clone();
            join_set.spawn(async move {
                // ERR-5 / TASK-0210: a closed semaphore panicked the worker
                // with expect("semaphore closed"), yielding a generic
                // <panicked> StepResult. Since the semaphore is scoped to
                // the parent spawn frame it can never be closed while a
                // child holds an Arc to it — but rather than encode that
                // invariant via `expect`, surface a descriptive failure
                // so any future refactor that does drop the semaphore
                // shows up as a clear error instead of a panic.
                let permit = match sem.acquire().await {
                    Ok(p) => p,
                    Err(_) => {
                        return StepResult::failure(
                            task_id.as_str(),
                            Duration::ZERO,
                            "internal error: parallel semaphore closed before task could acquire a permit"
                                .to_string(),
                        );
                    }
                };
                let _permit = permit;
                exec_standalone(id, spec, cwd, vars, tx, abort).await
            });
        }
        drop(tx);
        (rx, abort, join_set)
    }

    /// Run a flat list of exec command IDs in parallel; events sent via channel. When fail_fast is true, abort flag is set on first failure.
    ///
    /// # Resource Considerations
    ///
    /// All commands are spawned concurrently. For configs with many parallel commands,
    /// this may consume significant system resources (file descriptors, memory, CPU).
    /// Consider splitting large parallel groups into smaller batches if resource
    /// exhaustion is a concern.
    #[instrument(skip(self, on_event))]
    pub async fn run_plan_parallel(
        &self,
        command_ids: &[CommandId],
        fail_fast: bool,
        on_event: &mut impl FnMut(RunnerEvent),
    ) -> Vec<StepResult> {
        let lifecycle = PlanLifecycle::begin(command_ids, on_event);

        let steps = match self.resolve_exec_specs(command_ids) {
            Ok(s) => s,
            Err(id) => {
                let msg = format!("internal error: composite in leaf plan: {}", id);
                let results = vec![resolution_failure(&id, msg, on_event)];
                lifecycle.finish(results.iter().all(|r| r.success), on_event);
                return results;
            }
        };

        let (rx, abort, mut join_set) =
            Self::spawn_parallel_tasks(steps, self.cwd.clone(), self.vars.clone());
        // CONC-6 / TASK-0204: when fail_fast sees the first failure, set
        // the abort flag **and** actively `abort_all()` the JoinSet so
        // siblings stop rendering output. Previously the loop kept
        // draining rx until every tx dropped, so a 5s sibling kept
        // emitting events long after the 100ms failure that triggered
        // fail_fast. Pass a JoinSet handle to `handle_parallel_events` so
        // it can cancel in-flight work.
        Self::handle_parallel_events_with_cancel(rx, fail_fast, abort, &mut join_set, on_event)
            .await;
        let results = Self::collect_join_results(join_set).await;

        lifecycle.finish(results.iter().all(|r| r.success), on_event);
        results
    }

    /// Drain events, flipping `abort` on first failure under `fail_fast`.
    /// Kept as a thin wrapper around `handle_parallel_events_with_cancel`
    /// (passing a disposable empty JoinSet) so older tests keep working;
    /// production uses the cancelling variant.
    #[cfg(test)]
    pub(crate) async fn handle_parallel_events(
        rx: mpsc::Receiver<RunnerEvent>,
        fail_fast: bool,
        abort: Arc<AtomicBool>,
        on_event: &mut impl FnMut(RunnerEvent),
    ) {
        let mut empty: tokio::task::JoinSet<StepResult> = tokio::task::JoinSet::new();
        Self::handle_parallel_events_with_cancel(rx, fail_fast, abort, &mut empty, on_event).await;
    }

    /// Drain events, and on first failure under `fail_fast` abort any
    /// in-flight parallel tasks via `JoinSet::abort_all`.
    pub(crate) async fn handle_parallel_events_with_cancel(
        mut rx: mpsc::Receiver<RunnerEvent>,
        fail_fast: bool,
        abort: Arc<AtomicBool>,
        join_set: &mut tokio::task::JoinSet<StepResult>,
        on_event: &mut impl FnMut(RunnerEvent),
    ) {
        let mut cancelled = false;
        while let Some(ev) = rx.recv().await {
            if let RunnerEvent::StepFailed { .. } = &ev {
                if fail_fast && !cancelled {
                    abort.store(true, Ordering::Release);
                    join_set.abort_all();
                    cancelled = true;
                }
            }
            on_event(ev);
        }
    }

    /// Run a named command (single or composite); returns step results.
    pub async fn run(
        &self,
        command_id: &str,
        on_event: &mut impl FnMut(RunnerEvent),
    ) -> anyhow::Result<Vec<StepResult>> {
        let spec = self
            .resolve(command_id)
            .ok_or_else(|| ExpandError::Unknown(command_id.to_string()))?;
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

/// Build an `alias → canonical_name` map by flattening one or more command
/// stores in iteration order. Later stores override earlier ones (matching
/// the existing stack → extension precedence).
fn build_alias_map<'a, I>(stores: I) -> std::collections::HashMap<String, String>
where
    I: IntoIterator<Item = &'a IndexMap<CommandId, CommandSpec>>,
{
    let mut map = std::collections::HashMap::new();
    for store in stores {
        for (name, spec) in store {
            for alias in spec.aliases() {
                map.insert(alias.clone(), name.to_string());
            }
        }
    }
    map
}

#[cfg(test)]
mod tests;
