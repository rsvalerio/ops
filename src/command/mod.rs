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

use crate::config::{CommandId, CommandSpec, Config, ExecCommandSpec, OutputConfig};
use crate::extension::{DataProviderError, DataRegistry};
use crate::stack::Stack;
use exec::{exec_command, exec_standalone, resolution_failure};
use indexmap::IndexMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, instrument};

/// Runs commands from config; emits RunnerEvent stream.
pub struct CommandRunner {
    config: Arc<Config>,
    cwd: PathBuf,
    stack_commands: IndexMap<CommandId, CommandSpec>,
    extension_commands: std::collections::HashMap<CommandId, CommandSpec>,
    data_registry: DataRegistry,
    #[allow(dead_code)]
    data_cache: std::collections::HashMap<String, Arc<serde_json::Value>>,
    #[allow(dead_code)]
    detected_stack: Option<Stack>,
}

impl CommandRunner {
    pub fn new(config: Config, cwd: PathBuf) -> Self {
        let detected_stack = config
            .stack
            .as_deref()
            .and_then(Stack::from_str)
            .or_else(|| Stack::detect(&cwd));

        let stack_commands = if let Some(stack) = detected_stack {
            let defaults = stack.default_commands();
            debug!(
                stack = stack.as_str(),
                command_count = defaults.len(),
                "loaded stack default commands"
            );
            defaults
        } else {
            IndexMap::new()
        };

        Self {
            config: Arc::new(config),
            cwd,
            stack_commands,
            extension_commands: std::collections::HashMap::new(),
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

    /// Detected or configured stack.
    #[allow(dead_code)]
    pub fn stack(&self) -> Option<Stack> {
        self.detected_stack
    }

    /// Replace the internal data registry (e.g. with one populated by extensions).
    pub fn register_data_providers(&mut self, registry: DataRegistry) {
        self.data_registry = registry;
    }

    /// Query cached data or compute via provider.
    ///
    /// EFF-002: Uses entry API to avoid double-clone of data_cache.
    #[allow(dead_code)]
    pub fn query_data(&mut self, name: &str) -> Result<Arc<serde_json::Value>, DataProviderError> {
        use std::collections::hash_map::Entry;
        match self.data_cache.entry(name.to_string()) {
            Entry::Occupied(v) => Ok(Arc::clone(v.get())),
            Entry::Vacant(slot) => {
                let mut ctx =
                    crate::extension::Context::new(Arc::clone(&self.config), self.cwd.clone());
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

    /// Resolve a command by ID (config first, then stack defaults, then extension).
    pub fn resolve(&self, id: &str) -> Option<&CommandSpec> {
        self.config
            .commands
            .get(id)
            .or(self.stack_commands.get(id))
            .or(self.extension_commands.get(id))
    }

    /// List all available command IDs (config first, then stack, then extension commands; sorted for stable order).
    pub fn list_command_ids(&self) -> Vec<CommandId> {
        let mut ids: Vec<&str> = self.config.commands.keys().map(|s| s.as_str()).collect();
        ids.extend(self.stack_commands.keys().map(|s| s.as_str()));
        ids.extend(self.extension_commands.keys().map(|s| s.as_str()));
        ids.sort_unstable();
        ids.dedup();
        ids.iter().map(|s| s.to_string()).collect()
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
        /// nested composites (e.g., a → b → c → ... → z with 100+ levels). Normal
        /// configs typically have 2-5 levels (e.g., verify → [build, test] → cargo).
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
        let spec = self.resolve(id)?;
        match spec {
            CommandSpec::Exec(_) => Some(vec![id.to_string()]),
            CommandSpec::Composite(c) => {
                if !visited.insert(id.to_string()) {
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
        exec_command(id, spec, &self.cwd, on_event).await
    }

    /// Run a flat list of exec command IDs sequentially; stop on first failure.
    #[instrument(skip(self, on_event))]
    pub async fn run_plan(
        &self,
        command_ids: &[CommandId],
        on_event: &mut impl FnMut(RunnerEvent),
    ) -> Vec<StepResult> {
        on_event(RunnerEvent::PlanStarted {
            command_ids: command_ids.to_vec(),
        });
        let start = Instant::now();
        let mut results = Vec::new();

        for id in command_ids {
            let spec = match self.resolve(id) {
                Some(s) => s,
                None => {
                    let msg = format!("unknown command: {}", id);
                    results.push(resolution_failure(id, msg, on_event));
                    break;
                }
            };

            match spec {
                CommandSpec::Exec(e) => {
                    let r = self.run_exec(id, e, on_event).await;
                    let failed = !r.success;
                    results.push(r);
                    if failed {
                        break;
                    }
                }
                CommandSpec::Composite(_) => {
                    let msg = format!("internal error: composite in leaf plan: {}", id);
                    results.push(resolution_failure(id, msg, on_event));
                    break;
                }
            }
        }

        let success = results.iter().all(|r| r.success);
        on_event(RunnerEvent::RunFinished {
            duration_secs: start.elapsed().as_secs_f64(),
            success,
        });
        results
    }

    /// Resolve command IDs to exec specs, returning Err with the offending ID on failure.
    fn resolve_exec_specs(
        &self,
        command_ids: &[CommandId],
    ) -> Result<Vec<(CommandId, ExecCommandSpec)>, CommandId> {
        let mut steps = Vec::with_capacity(command_ids.len());
        for id in command_ids {
            match self.resolve(id) {
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
    async fn collect_join_results(
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

    /// Spawn parallel tasks into a JoinSet, returning the receiver and abort flag.
    ///
    /// # Memory Considerations
    ///
    /// Uses an unbounded channel for event streaming. This is acceptable because:
    /// - Events are consumed immediately by the main task
    /// - Each event is small (typically < 1KB)
    /// - The number of parallel commands is typically small (< 100)
    ///
    /// If memory becomes a concern with very large parallel groups,
    /// consider splitting into smaller batches or adding a config option
    /// for maximum parallelism.
    fn spawn_parallel_tasks(
        steps: Vec<(CommandId, ExecCommandSpec)>,
        cwd: PathBuf,
    ) -> (
        mpsc::UnboundedReceiver<RunnerEvent>,
        Arc<AtomicBool>,
        tokio::task::JoinSet<StepResult>,
    ) {
        let (tx, rx) = mpsc::unbounded_channel();
        let abort = Arc::new(AtomicBool::new(false));
        let mut join_set = tokio::task::JoinSet::new();
        for (id, spec) in steps {
            let tx = tx.clone();
            let abort = Arc::clone(&abort);
            let cwd_clone = cwd.clone();
            join_set.spawn(async move { exec_standalone(id, spec, cwd_clone, tx, abort).await });
        }
        drop(tx);
        (rx, abort, join_set)
    }

    /// Receive events from parallel execution, handling fail_fast abort.
    async fn handle_parallel_events(
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

    #[cfg(test)]
    pub fn spawn_parallel_tasks_for_test(
        steps: Vec<(CommandId, ExecCommandSpec)>,
        cwd: PathBuf,
    ) -> (
        mpsc::UnboundedReceiver<RunnerEvent>,
        Arc<AtomicBool>,
        tokio::task::JoinSet<StepResult>,
    ) {
        Self::spawn_parallel_tasks(steps, cwd)
    }

    #[cfg(test)]
    pub async fn handle_parallel_events_for_test(
        rx: mpsc::UnboundedReceiver<RunnerEvent>,
        fail_fast: bool,
        abort: Arc<AtomicBool>,
        on_event: &mut impl FnMut(RunnerEvent),
    ) {
        Self::handle_parallel_events(rx, fail_fast, abort, on_event).await
    }

    #[cfg(test)]
    pub async fn collect_join_results_for_test(
        join_set: tokio::task::JoinSet<StepResult>,
    ) -> Vec<StepResult> {
        Self::collect_join_results(join_set).await
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
        on_event(RunnerEvent::PlanStarted {
            command_ids: command_ids.to_vec(),
        });
        let start = Instant::now();

        let steps = match self.resolve_exec_specs(command_ids) {
            Ok(s) => s,
            Err(id) => {
                let msg = "internal error: composite in leaf plan".to_string();
                let result = resolution_failure(&id, msg, on_event);
                on_event(RunnerEvent::RunFinished {
                    duration_secs: start.elapsed().as_secs_f64(),
                    success: false,
                });
                return vec![result];
            }
        };

        let (rx, abort, join_set) = Self::spawn_parallel_tasks(steps, self.cwd.clone());
        Self::handle_parallel_events(rx, fail_fast, abort, on_event).await;
        let results = Self::collect_join_results(join_set).await;

        let success = results.iter().all(|r| r.success);
        on_event(RunnerEvent::RunFinished {
            duration_secs: start.elapsed().as_secs_f64(),
            success,
        });
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
            _ => self.run_plan(&plan, on_event).await,
        };
        Ok(results)
    }
}

#[cfg(test)]
mod tests;
