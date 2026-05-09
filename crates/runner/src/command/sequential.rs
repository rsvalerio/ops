//! Sequential plan execution: per-step exec dispatch and `run_plan` /
//! `run_plan_raw` / `run_raw` orchestration.
//!
//! Split out of `command/mod.rs` (ARCH-1 / TASK-0303) alongside `parallel`.

use super::events::PlanLifecycle;
use super::exec::{exec_command_raw, resolution_failure};
use super::{CommandRunner, RunnerEvent, StepResult};
use ops_core::config::{CommandId, CommandSpec};
use std::time::Duration;
use tracing::{debug, instrument};

impl CommandRunner {
    /// Execute a single step in a sequential plan, returning the result and whether to stop.
    async fn execute_step(
        &self,
        id: &str,
        on_event: &mut impl FnMut(RunnerEvent),
    ) -> (StepResult, bool) {
        match self.resolve_exec_leaf(id) {
            Ok(e) => {
                // PERF-3 / TASK-1125: wrap once at the boundary; downstream
                // build_command_async dispatch is then Arc::clone, not deep clone.
                let e = std::sync::Arc::new(e);
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
            // PERF-3 / TASK-1125: wrap once at the boundary; build_command_async
            // dispatch is then Arc::clone, not deep clone of args/env.
            let spec = std::sync::Arc::new(spec);
            let result = exec_command_raw(
                id.as_str(),
                &spec,
                &self.workspace_cache,
                &self.cwd,
                &self.vars,
                self.cwd_escape_policy,
            )
            .await;
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
}
