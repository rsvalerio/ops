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
pub use results::StepResult;

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

    fn finish(self, results: &[StepResult], on_event: &mut impl FnMut(RunnerEvent)) {
        let success = results.iter().all(|r| r.success);
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

        Self {
            config: Arc::new(config),
            cwd,
            vars,
            stack_commands,
            extension_commands: IndexMap::new(),
            data_registry: DataRegistry::new(),
            data_cache: std::collections::HashMap::new(),
            detected_stack,
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
    }

    /// Stack and extension command sources (config excluded; its aliases use a dedicated map).
    fn non_config_sources(&self) -> [&IndexMap<CommandId, CommandSpec>; 2] {
        [&self.stack_commands, &self.extension_commands]
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
    fn canonical_id<'a>(&'a self, id: &'a str) -> &'a str {
        if self.exists_in_stores(id) {
            return id;
        }
        if let Some(name) = self.config.resolve_alias(id) {
            return name;
        }
        for src in self.non_config_sources() {
            for (name, spec) in src {
                if spec.aliases().iter().any(|a| a == id) {
                    return name.as_str();
                }
            }
        }
        id
    }

    /// Look up a command by alias across all command sources.
    fn resolve_alias(&self, alias: &str) -> Option<&CommandSpec> {
        // Config aliases use a dedicated method (separate alias map)
        if let Some(name) = self.config.resolve_alias(alias) {
            return self.config.commands.get(name);
        }
        self.non_config_sources().into_iter().find_map(|src| {
            src.values()
                .find(|spec| spec.aliases().iter().any(|a| a == alias))
        })
    }

    /// List all available command IDs (config first, then stack, then extension commands; sorted for stable order).
    pub fn list_command_ids(&self) -> Vec<CommandId> {
        let mut ids: Vec<&str> = self.all_command_keys().collect();
        ids.sort_unstable();
        ids.dedup();
        ids.iter().map(|s| CommandId::from(*s)).collect()
    }

    /// Expand to a flat list of exec-only command IDs (no composites), so run_plan need not recurse.
    /// Returns `None` if any referenced command is unknown or a cycle is detected.
    ///
    /// # Recursion Depth
    ///
    /// The recursion is bounded by the cycle detection mechanism - each composite can only
    /// be visited once per expansion. For deeply nested composites, the call stack depth is
    /// limited by the number of unique composites, not the total depth. In practice, this
    /// means a graph with N composites has at most N stack frames during expansion.
    ///
    /// An additional guard limits expansion to 100 levels to prevent pathological cases.
    pub fn expand_to_leaves(&self, id: &str) -> Option<Vec<CommandId>> {
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
    ) -> Option<Vec<CommandId>> {
        if depth > max_depth {
            tracing::warn!(
                id = %id,
                depth = depth,
                max_depth = max_depth,
                "composite expansion depth limit exceeded"
            );
            return None;
        }
        let canonical = self.canonical_id(id);
        let spec = self.resolve(canonical)?;
        match spec {
            CommandSpec::Exec(_) => Some(vec![CommandId::from(canonical)]),
            CommandSpec::Composite(c) => {
                if !visited.insert(canonical.to_string()) {
                    return None; // cycle detected
                }
                let mut out = Vec::new();
                for sub in &c.commands {
                    out.extend(self.expand_inner(sub, visited, depth + 1, max_depth)?);
                }
                Some(out)
            }
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
        let Some(spec) = self.resolve(id) else {
            let msg = format!("unknown command: {}", id);
            return (resolution_failure(id, msg, on_event), true);
        };
        match spec {
            CommandSpec::Exec(e) => {
                let r = self.run_exec(id, e, on_event).await;
                let should_stop = !r.success;
                (r, should_stop)
            }
            CommandSpec::Composite(_) => {
                let msg = format!("internal error: composite in leaf plan: {}", id);
                (resolution_failure(id, msg, on_event), true)
            }
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

        lifecycle.finish(&results, on_event);
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
            let spec = match self.resolve(id.as_str()) {
                Some(CommandSpec::Exec(e)) => e.clone(),
                Some(CommandSpec::Composite(_)) => {
                    results.push(StepResult::failure(
                        id.as_str(),
                        Duration::ZERO,
                        format!("internal error: composite in leaf plan: {}", id),
                    ));
                    if fail_fast {
                        break;
                    }
                    continue;
                }
                None => {
                    results.push(StepResult::failure(
                        id.as_str(),
                        Duration::ZERO,
                        format!("unknown command: {}", id),
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
            .ok_or_else(|| anyhow::anyhow!("unknown command: {}", command_id))?;
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
                    tracing::error!("parallel task panicked: {}", e);
                    results.push(StepResult::failure(
                        "<panicked>",
                        Duration::ZERO,
                        format!("task panicked: {}", e),
                    ));
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
    pub(crate) fn spawn_parallel_tasks(
        steps: Vec<(CommandId, ExecCommandSpec)>,
        cwd: PathBuf,
        vars: Variables,
    ) -> (
        mpsc::UnboundedReceiver<RunnerEvent>,
        Arc<AtomicBool>,
        tokio::task::JoinSet<StepResult>,
    ) {
        // Unbounded channel: events must not be dropped because lost
        // StepFinished/StepFailed events corrupt the progress display, and
        // tracing::warn! on drop conflicts with indicatif's stderr control.
        // Memory is bounded by actual subprocess output which is finite.
        let (tx, rx) = mpsc::unbounded_channel();
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
            join_set.spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore closed");
                exec_standalone(id, spec, cwd, vars, tx, abort).await
            });
        }
        drop(tx);
        (rx, abort, join_set)
    }

    /// Receive events from parallel execution, handling fail_fast abort.
    pub(crate) async fn handle_parallel_events(
        mut rx: mpsc::UnboundedReceiver<RunnerEvent>,
        fail_fast: bool,
        abort: Arc<AtomicBool>,
        on_event: &mut impl FnMut(RunnerEvent),
    ) {
        while let Some(ev) = rx.recv().await {
            if let RunnerEvent::StepFailed { .. } = &ev {
                if fail_fast {
                    abort.store(true, Ordering::Release);
                }
            }
            on_event(ev);
        }
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
                lifecycle.finish(&results, on_event);
                return results;
            }
        };

        let (rx, abort, join_set) =
            Self::spawn_parallel_tasks(steps, self.cwd.clone(), self.vars.clone());
        Self::handle_parallel_events(rx, fail_fast, abort, on_event).await;
        let results = Self::collect_join_results(join_set).await;

        lifecycle.finish(&results, on_event);
        results
    }

    /// Run a named command (single or composite); returns step results.
    pub async fn run(
        &self,
        command_id: &str,
        on_event: &mut impl FnMut(RunnerEvent),
    ) -> anyhow::Result<Vec<StepResult>> {
        let spec = self
            .resolve(command_id)
            .ok_or_else(|| anyhow::anyhow!("unknown command: {}", command_id))?;
        let plan = self
            .expand_to_leaves(command_id)
            .ok_or_else(|| anyhow::anyhow!("unknown command: {}", command_id))?;
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
