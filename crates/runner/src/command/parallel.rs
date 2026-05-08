//! Parallel exec orchestration: bounded mpsc channel, fail-fast cancellation,
//! `JoinSet` collection.
//!
//! Split out of `command/mod.rs` (ARCH-1 / TASK-0303) so the orchestrator
//! file isn't carrying both sequential and parallel scheduling concerns.

use super::abort::AbortSignal;
use super::build::CwdEscapePolicy;
use super::events::PlanLifecycle;
use super::exec::{exec_standalone, resolution_failure, ExecTaskCtx};
use super::{CommandRunner, RunnerEvent, StepResult};
use ops_core::config::{CommandId, ExecCommandSpec};
use ops_core::expand::Variables;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::Id as TaskId;
use tracing::instrument;

/// Default cap on concurrent parallel exec tasks.
///
/// CONC-3 / TASK-0873: overridable via `OPS_MAX_PARALLEL`. The default
/// (32) is generous for developer machines; CI runners with tight FD or
/// process limits can dial it down without recompiling.
const DEFAULT_MAX_PARALLEL: usize = 32;

/// Per-parallel-task event budget used to size the bounded event channel.
///
/// Budget = StepStarted + N×StepOutput + (StepFinished | StepFailed |
/// StepSkipped). Real commands rarely hit N=256 between display pumps;
/// when a burst does fill the channel the producer task awaits on
/// `send`, which naturally back-pressures chatty children instead of
/// letting the process drift toward OOM.
///
/// CONC-3 / TASK-0873: overridable via `OPS_PARALLEL_EVENT_BUDGET`.
const DEFAULT_PARALLEL_EVENT_BUDGET_PER_TASK: usize = 256;

/// Hard ceiling on the env-overridable parallel cap. Rejects pathological
/// values (e.g. `OPS_MAX_PARALLEL=1000000`) that would defeat the
/// resource-pressure contract this knob exists to enforce.
pub(crate) const MAX_PARALLEL_CEILING: usize = 1024;

/// Hard ceiling on the env-overridable per-task event budget. Same
/// rationale as [`MAX_PARALLEL_CEILING`].
const MAX_EVENT_BUDGET_CEILING: usize = 65_536;

/// Resolve [`DEFAULT_MAX_PARALLEL`] honoring `OPS_MAX_PARALLEL`. Invalid,
/// zero, or above-ceiling values fall back to the default with a
/// `tracing::warn!` so misconfiguration is visible.
///
/// PERF-3 / TASK-0995: exposed to `command::results` so the
/// `output_byte_cap` peak-RSS warning is computed against the same
/// (clamped, validated) value the orchestrator actually uses, instead of
/// silently re-parsing the raw env var with different fallback rules.
pub(crate) fn resolve_max_parallel() -> usize {
    resolve_env_usize(
        "OPS_MAX_PARALLEL",
        DEFAULT_MAX_PARALLEL,
        MAX_PARALLEL_CEILING,
    )
}

fn resolve_event_budget() -> usize {
    resolve_env_usize(
        "OPS_PARALLEL_EVENT_BUDGET",
        DEFAULT_PARALLEL_EVENT_BUDGET_PER_TASK,
        MAX_EVENT_BUDGET_CEILING,
    )
}

/// ERR-1 / TASK-1092: the warn-message text emitted when `OPS_MAX_PARALLEL`
/// (or `OPS_PARALLEL_EVENT_BUDGET`) is set to `0`. Pinned as a `const` so
/// a unit test can assert the operator-facing diagnostic — distinguishing
/// "explicit 0 (sequential intent)" from a generic parse failure — does
/// not silently regress.
pub(crate) const ZERO_NOT_ALLOWED_MSG: &str =
    "zero is not allowed; use 1 for sequential execution; falling back to default";

fn resolve_env_usize(var: &'static str, default: usize, ceiling: usize) -> usize {
    let Ok(raw) = std::env::var(var) else {
        return default;
    };
    // ERR-1 / TASK-1092: empty string (e.g. `OPS_MAX_PARALLEL=`) is
    // treated the same as unset — operators clearing the variable should
    // not see a confusing `value = ""` parse-error warning.
    if raw.is_empty() {
        return default;
    }
    match raw.parse::<usize>() {
        Ok(0) => {
            // ERR-1 / TASK-1092: distinguish "user explicitly asked for
            // sequential by setting 0" from "garbage value". Zero is a
            // valid intent but not a legal channel/semaphore size, so we
            // still fall back to the default — but say so explicitly so
            // operators know to use `1` for single-threaded execution.
            tracing::warn!(env = var, default, "{ZERO_NOT_ALLOWED_MSG}");
            default
        }
        Err(_) => {
            tracing::warn!(
                env = var,
                value = %raw,
                default,
                "unparseable value; falling back to default"
            );
            default
        }
        Ok(n) if n > ceiling => {
            tracing::warn!(
                env = var,
                requested = n,
                ceiling,
                "clamping to ceiling; the bounded contract is the knob's purpose"
            );
            ceiling
        }
        Ok(n) => n,
    }
}

impl CommandRunner {
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
    #[cfg(test)]
    pub(crate) async fn collect_join_results(
        join_set: tokio::task::JoinSet<StepResult>,
        id_map: &HashMap<TaskId, CommandId>,
    ) -> Vec<StepResult> {
        Self::collect_join_results_with_pre(Vec::new(), join_set, id_map).await
    }

    /// CONC-6 / TASK-1177: merge results harvested by
    /// `handle_parallel_events_with_cancel_inner` (which already drained
    /// completed tasks during the events loop so panics could trigger
    /// fail_fast) with whatever still remains in the JoinSet.
    pub(crate) async fn collect_join_results_with_pre(
        pre: Vec<(TaskId, Result<StepResult, tokio::task::JoinError>)>,
        mut join_set: tokio::task::JoinSet<StepResult>,
        id_map: &HashMap<TaskId, CommandId>,
    ) -> Vec<StepResult> {
        let mut results = Vec::new();
        for (task_id, res) in pre {
            Self::push_one(&mut results, task_id, res, id_map);
        }
        // READ-5 / TASK-0767: use `join_next_with_id` so a panicking task
        // surfaces the originating `CommandId` (looked up via `id_map`)
        // instead of a sentinel "<panicked>". JSON event consumers and CI
        // dashboards can then correlate the panicked StepResult with the
        // step that produced it.
        while let Some(res) = join_set.join_next_with_id().await {
            let (task_id, joined): (TaskId, Result<StepResult, tokio::task::JoinError>) = match res
            {
                Ok((tid, sr)) => (tid, Ok(sr)),
                Err(e) => (e.id(), Err(e)),
            };
            Self::push_one(&mut results, task_id, joined, id_map);
        }
        results
    }

    fn push_one(
        results: &mut Vec<StepResult>,
        _task_id: TaskId,
        res: Result<StepResult, tokio::task::JoinError>,
        id_map: &HashMap<TaskId, CommandId>,
    ) {
        match res {
            Ok(step_result) => results.push(step_result),
            Err(e) => {
                let cmd_id = id_map
                    .get(&e.id())
                    .cloned()
                    .unwrap_or_else(|| CommandId::from("<panicked>"));
                // CONC-6 / TASK-0214: distinguish a cancellation
                // (fail_fast aborted the JoinSet) from a real panic so
                // users see "cancelled" rather than misleading
                // "panicked" for siblings that were intentionally
                // stopped.
                if e.is_cancelled() {
                    tracing::debug!(id = %cmd_id, "parallel task cancelled (fail_fast abort)");
                    results.push(StepResult::cancelled(cmd_id));
                } else {
                    // SEC-21 / TASK-0334: a JoinError's Display embeds the
                    // panic payload, which often contains attacker-influenced
                    // data (absolute paths from `expect`/`unwrap` panics,
                    // user-supplied strings). That message flows into
                    // StepResult.message → StepFailed → tap file / TAP CI
                    // output, mirroring the leak channel SEC-22 closed for
                    // spawn errors. Surface a generic message and log the
                    // raw payload at debug for operators.
                    tracing::debug!(id = %cmd_id, error = %e, "parallel task panicked (full payload)");
                    results.push(StepResult::failure(
                        cmd_id,
                        Duration::ZERO,
                        "task panicked".to_string(),
                    ));
                }
            }
        }
    }

    /// Spawn parallel tasks into a JoinSet, returning the receiver and abort flag.
    ///
    /// Concurrency is capped at `MAX_PARALLEL` via a semaphore to prevent
    /// resource exhaustion with large parallel groups.
    pub(crate) fn spawn_parallel_tasks(
        steps: Vec<(CommandId, ExecCommandSpec)>,
        cwd: Arc<PathBuf>,
        vars: Arc<Variables>,
        policy: CwdEscapePolicy,
        workspace_cache: Arc<super::build::WorkspaceCanonicalCache>,
    ) -> (
        mpsc::Receiver<RunnerEvent>,
        Arc<AbortSignal>,
        tokio::task::JoinSet<StepResult>,
        HashMap<TaskId, CommandId>,
    ) {
        // CONC-3 / TASK-0158+0209: bounded channel so a chatty child
        // back-pressures on the display pump instead of growing the mpsc
        // buffer until the process OOMs. Capacity is sized to
        // MAX_PARALLEL × per-task event budget so the steady-state batch
        // of events never blocks; only pathological bursts of >N lines
        // per tick will pause a producer — which is exactly the
        // throttling we want.
        let max_parallel = resolve_max_parallel();
        let event_budget = resolve_event_budget();
        let capacity = max_parallel.saturating_mul(event_budget);
        let (tx, rx) = mpsc::channel(capacity);
        let abort = Arc::new(AbortSignal::new());
        let semaphore = Arc::new(tokio::sync::Semaphore::new(max_parallel));
        let mut join_set = tokio::task::JoinSet::new();
        let mut id_map: HashMap<TaskId, CommandId> = HashMap::new();
        for (id, spec) in steps {
            let tx = tx.clone();
            let abort = Arc::clone(&abort);
            let cwd = Arc::clone(&cwd);
            let vars = Arc::clone(&vars);
            let cache = Arc::clone(&workspace_cache);
            let sem = Arc::clone(&semaphore);
            let task_id = id.clone();
            let cmd_id = id.clone();
            let handle = join_set.spawn(async move {
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
                exec_standalone(
                    id,
                    spec,
                    ExecTaskCtx {
                        cwd,
                        vars,
                        tx,
                        abort,
                        policy,
                        workspace_cache: cache,
                    },
                )
                .await
            });
            // READ-5 / TASK-0767: remember which tokio task carries which
            // CommandId so `collect_join_results` can preserve the id even
            // when the task panics (JoinError carries the task `Id`, not the
            // CommandId).
            id_map.insert(handle.id(), cmd_id);
        }
        drop(tx);
        (rx, abort, join_set, id_map)
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
        // ASYNC-7 / TASK-0777: parallel orchestration (channel + JoinSet +
        // AbortSignal + forwarder) only pays off when there are at least two
        // tasks to overlap. For command_ids.len() <= 1, delegate to the
        // sequential `run_plan` path: identical observable semantics
        // (PlanStarted → step events → RunFinished), no orchestration
        // overhead, no fresh `Arc`/channel allocations on the hot path.
        // Threshold is documented next to `DEFAULT_MAX_PARALLEL` above.
        if command_ids.len() <= 1 {
            return self.run_plan(command_ids, fail_fast, on_event).await;
        }
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

        let (rx, abort, mut join_set, id_map) = Self::spawn_parallel_tasks(
            steps,
            self.cwd.clone(),
            self.vars.clone(),
            self.cwd_escape_policy,
            Arc::clone(&self.workspace_cache),
        );
        // CONC-6 / TASK-0204: when fail_fast sees the first failure, set
        // the abort flag **and** actively `abort_all()` the JoinSet so
        // siblings stop rendering output. Previously the loop kept
        // draining rx until every tx dropped, so a 5s sibling kept
        // emitting events long after the 100ms failure that triggered
        // fail_fast. Pass a JoinSet handle to `handle_parallel_events` so
        // it can cancel in-flight work.
        // ERR-1 / TASK-0768: track terminal-event counts per plan command
        // id so we can synthesize a `StepSkipped` for any orphan whose
        // task was aborted before its terminal event reached the channel.
        //
        // PATTERN-1 / TASK-0997: count occurrences instead of using a
        // `HashSet`. A composite that fans the same leaf id twice (legal
        // — `expand_to_leaves` only guards cycles, not duplicates) and
        // aborts both before either terminal event arrived would
        // otherwise emit just one `StepSkipped`, leaving the second
        // `StepStarted` unpaired in JSON event consumers and the display.
        let mut terminal_counts: std::collections::HashMap<CommandId, usize> =
            std::collections::HashMap::new();
        let mut harvested: Vec<(TaskId, Result<StepResult, tokio::task::JoinError>)> = Vec::new();
        {
            let on_event_inner: &mut dyn FnMut(RunnerEvent) = on_event;
            let mut wrapped = |ev: RunnerEvent| {
                match &ev {
                    RunnerEvent::StepFinished { id, .. }
                    | RunnerEvent::StepFailed { id, .. }
                    | RunnerEvent::StepSkipped { id, .. } => {
                        *terminal_counts.entry(id.clone()).or_insert(0) += 1;
                    }
                    _ => {}
                }
                on_event_inner(ev);
            };
            // CONC-6 / TASK-1177: route through the inner variant so a
            // panicked sibling is observed during the events loop and
            // trips fail_fast at the same point a `StepFailed` would,
            // instead of leaking a 5s sibling that keeps emitting events
            // until the channel drains naturally.
            Self::handle_parallel_events_with_cancel_inner(
                rx,
                fail_fast,
                abort,
                &mut join_set,
                &mut wrapped,
                &mut harvested,
            )
            .await;
        }
        // Walk the plan once, decrementing the per-id seen count: each
        // visited slot either consumes one observed terminal event or
        // emits a synthetic `StepSkipped` for that occurrence. This keeps
        // total terminal-event count equal to `command_ids.len()` even
        // when ids repeat.
        for id in command_ids {
            let entry = terminal_counts.entry(id.clone()).or_insert(0);
            if *entry > 0 {
                *entry -= 1;
            } else {
                on_event(RunnerEvent::StepSkipped {
                    id: id.clone(),
                    display_cmd: None,
                });
            }
        }
        let results = Self::collect_join_results_with_pre(harvested, join_set, &id_map).await;

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
        abort: Arc<AbortSignal>,
        on_event: &mut impl FnMut(RunnerEvent),
    ) {
        let mut empty: tokio::task::JoinSet<StepResult> = tokio::task::JoinSet::new();
        Self::handle_parallel_events_with_cancel(rx, fail_fast, abort, &mut empty, on_event).await;
    }

    /// Drain events, and on first failure under `fail_fast` abort any
    /// in-flight parallel tasks via `JoinSet::abort_all`.
    ///
    /// CONC-6 / TASK-1177: also poll the JoinSet alongside the events
    /// channel so a task that **panics** (rather than returning a non-zero
    /// exit code) trips fail_fast at the same point a `StepFailed` event
    /// would. Pre-fix, a panicked sibling surfaced only after the channel
    /// drained naturally — defeating fail_fast for the panic path while a
    /// 5-second sibling kept emitting output. Panic results captured here
    /// are stashed in `panic_results` and merged by [`collect_join_results`]
    /// so the caller still observes the synthesized failure.
    #[cfg(test)]
    pub(crate) async fn handle_parallel_events_with_cancel(
        rx: mpsc::Receiver<RunnerEvent>,
        fail_fast: bool,
        abort: Arc<AbortSignal>,
        join_set: &mut tokio::task::JoinSet<StepResult>,
        on_event: &mut impl FnMut(RunnerEvent),
    ) {
        let mut empty: Vec<(tokio::task::Id, Result<StepResult, tokio::task::JoinError>)> =
            Vec::new();
        Self::handle_parallel_events_with_cancel_inner(
            rx, fail_fast, abort, join_set, on_event, &mut empty,
        )
        .await;
        // The default test-facing wrapper does not surface harvested
        // results; production routes through `run_plan_parallel` which
        // calls the inner variant directly so it can merge them.
        for (_, r) in empty {
            drop(r);
        }
    }

    /// CONC-6 / TASK-1177: inner select-loop variant that also drains
    /// completed tasks from the JoinSet so panics trigger fail_fast.
    /// Harvested `(task_id, result)` pairs are appended to
    /// `harvested_results` so callers can merge them into the
    /// `collect_join_results` output without losing the panic-aware
    /// step ids `id_map` records.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn handle_parallel_events_with_cancel_inner(
        mut rx: mpsc::Receiver<RunnerEvent>,
        fail_fast: bool,
        abort: Arc<AbortSignal>,
        join_set: &mut tokio::task::JoinSet<StepResult>,
        on_event: &mut impl FnMut(RunnerEvent),
        harvested_results: &mut Vec<(tokio::task::Id, Result<StepResult, tokio::task::JoinError>)>,
    ) {
        let mut cancelled = false;
        let mut rx_open = true;
        loop {
            // Stop once the events channel closed AND the JoinSet is
            // empty — both are required to fully drain.
            if !rx_open && join_set.is_empty() {
                break;
            }
            tokio::select! {
                biased;
                ev = rx.recv(), if rx_open => {
                    match ev {
                        Some(ev) => {
                            if let RunnerEvent::StepFailed { .. } = &ev {
                                if fail_fast && !cancelled {
                                    abort.set();
                                    join_set.abort_all();
                                    cancelled = true;
                                }
                            }
                            on_event(ev);
                        }
                        None => {
                            rx_open = false;
                        }
                    }
                }
                joined = join_set.join_next_with_id(), if !join_set.is_empty() => {
                    match joined {
                        Some(Ok((task_id, result))) => {
                            harvested_results.push((task_id, Ok(result)));
                        }
                        Some(Err(join_err)) => {
                            // CONC-6 / TASK-1177: a JoinError that is NOT a
                            // cancellation surfaces a panicked task. Trip
                            // fail_fast so live siblings stop.
                            if !join_err.is_cancelled() && fail_fast && !cancelled {
                                abort.set();
                                join_set.abort_all();
                                cancelled = true;
                            }
                            harvested_results.push((join_err.id(), Err(join_err)));
                        }
                        None => {}
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod resolve_tests {
    use super::*;

    /// PERF-3 / TASK-0995: an out-of-range `OPS_MAX_PARALLEL` must clamp
    /// to [`MAX_PARALLEL_CEILING`] so the `output_byte_cap` peak-RSS
    /// warning (which now reuses this resolver) is computed against the
    /// same clamped value the orchestrator actually uses, not against the
    /// raw env var.
    #[serial_test::serial(env_max_parallel)]
    #[test]
    fn resolve_max_parallel_clamps_above_ceiling() {
        let prev = std::env::var_os("OPS_MAX_PARALLEL");
        // SAFETY: tests serialised via `serial_test` so other threads
        // cannot read the env mid-mutation.
        unsafe { std::env::set_var("OPS_MAX_PARALLEL", "5000") };
        let resolved = resolve_max_parallel();
        match prev {
            Some(v) => unsafe { std::env::set_var("OPS_MAX_PARALLEL", v) },
            None => unsafe { std::env::remove_var("OPS_MAX_PARALLEL") },
        }
        assert_eq!(
            resolved, MAX_PARALLEL_CEILING,
            "OPS_MAX_PARALLEL=5000 must clamp to {MAX_PARALLEL_CEILING}, not pass through; the peak-RSS warning depends on this"
        );
    }

    #[serial_test::serial(env_max_parallel)]
    #[test]
    fn resolve_max_parallel_falls_back_on_zero_or_unparseable() {
        let prev = std::env::var_os("OPS_MAX_PARALLEL");
        unsafe { std::env::set_var("OPS_MAX_PARALLEL", "junk") };
        let resolved_junk = resolve_max_parallel();
        unsafe { std::env::set_var("OPS_MAX_PARALLEL", "0") };
        let resolved_zero = resolve_max_parallel();
        match prev {
            Some(v) => unsafe { std::env::set_var("OPS_MAX_PARALLEL", v) },
            None => unsafe { std::env::remove_var("OPS_MAX_PARALLEL") },
        }
        assert_eq!(resolved_junk, DEFAULT_MAX_PARALLEL);
        assert_eq!(resolved_zero, DEFAULT_MAX_PARALLEL);
    }

    /// ERR-1 / TASK-1092 AC-1, AC-3: pin the operator-facing warn message
    /// for the zero case so the diagnostic
    /// ("use 1 for sequential execution") stays distinct from the generic
    /// parse-error message. A regression to the old joint
    /// "unparseable or zero value" wording would silently re-conflate
    /// "explicit sequential intent" with "garbage" — exactly the bug
    /// TASK-1092 closed.
    #[test]
    fn zero_warn_message_distinguishes_sequential_intent() {
        assert!(
            ZERO_NOT_ALLOWED_MSG.contains("zero is not allowed"),
            "operators must learn that 0 is rejected, not silently accepted: {ZERO_NOT_ALLOWED_MSG}"
        );
        assert!(
            ZERO_NOT_ALLOWED_MSG.contains("1 for sequential"),
            "the message must steer operators to the correct value (1) for sequential execution: {ZERO_NOT_ALLOWED_MSG}"
        );
        assert!(
            !ZERO_NOT_ALLOWED_MSG.contains("unparseable"),
            "the zero-case message must not conflate with the parse-error path: {ZERO_NOT_ALLOWED_MSG}"
        );
    }

    /// ERR-1 / TASK-1092 AC-2: an empty-string env var (`OPS_MAX_PARALLEL=`)
    /// is treated as unset, not as an "unparseable value = \"\"" warning.
    #[serial_test::serial(env_max_parallel)]
    #[test]
    fn resolve_max_parallel_treats_empty_as_unset() {
        let prev = std::env::var_os("OPS_MAX_PARALLEL");
        // SAFETY: tests serialised via `serial_test`.
        unsafe { std::env::set_var("OPS_MAX_PARALLEL", "") };
        let resolved = resolve_max_parallel();
        match prev {
            Some(v) => unsafe { std::env::set_var("OPS_MAX_PARALLEL", v) },
            None => unsafe { std::env::remove_var("OPS_MAX_PARALLEL") },
        }
        assert_eq!(
            resolved, DEFAULT_MAX_PARALLEL,
            "an explicitly-empty OPS_MAX_PARALLEL must behave like unset, not trip the parse-error fallback"
        );
    }
}
