//! Parallel exec orchestration: bounded mpsc channel, fail-fast cancellation,
//! `JoinSet` collection.
//!
//! Split out of `command/mod.rs` (ARCH-1 / TASK-0303) so the orchestrator
//! file isn't carrying both sequential and parallel scheduling concerns.

use super::events::PlanLifecycle;
use super::exec::{exec_standalone, resolution_failure};
use super::{CommandRunner, RunnerEvent, StepResult};
use ops_core::config::{CommandId, ExecCommandSpec};
use ops_core::expand::Variables;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::instrument;

impl CommandRunner {
    /// Maximum concurrent parallel tasks. Caps resource usage (file descriptors,
    /// processes) for configs with many parallel commands.
    const MAX_PARALLEL: usize = 32;

    /// Per-parallel-task event budget used to size the bounded event channel.
    ///
    /// Budget = StepStarted + N×StepOutput + (StepFinished | StepFailed |
    /// StepSkipped). Real commands rarely hit N=256 between display pumps;
    /// when a burst does fill the channel the producer task awaits on
    /// `send`, which naturally back-pressures chatty children instead of
    /// letting the process drift toward OOM.
    const PARALLEL_EVENT_BUDGET_PER_TASK: usize = 256;

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
                        results.push(StepResult::cancelled(CommandId::from("<cancelled>")));
                    } else {
                        // SEC-21 / TASK-0334: a JoinError's Display embeds the
                        // panic payload, which often contains attacker-influenced
                        // data (absolute paths from `expect`/`unwrap` panics,
                        // user-supplied strings). That message flows into
                        // StepResult.message → StepFailed → tap file / TAP CI
                        // output, mirroring the leak channel SEC-22 closed for
                        // spawn errors. Surface a generic message and log the
                        // raw payload at debug for operators.
                        tracing::debug!(error = %e, "parallel task panicked (full payload)");
                        results.push(StepResult::failure(
                            "<panicked>",
                            Duration::ZERO,
                            "task panicked".to_string(),
                        ));
                    }
                }
            }
        }
        results
    }

    /// Spawn parallel tasks into a JoinSet, returning the receiver and abort flag.
    ///
    /// Concurrency is capped at `MAX_PARALLEL` via a semaphore to prevent
    /// resource exhaustion with large parallel groups.
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
}
