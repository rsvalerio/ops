//! Matrix commands (TASK-2277): one exec step that runs once per cell.
//!
//! A command with `[commands.<name>.strategy]` is a single leaf to the plan
//! around it — scheduling (`exclusive` stages, the plan's `fail_fast`) treats
//! it as one step. This module runs that step: its cells under their own
//! schedule (`strategy.max_parallel`, `strategy.fail_fast`), each as an
//! ordinary exec with its own progress row, folded into one [`StepResult`]
//! for the plan.
//!
//! Cell events carry the cell's id (`name [key=value]`) and use it as their
//! display label, so every cell renders as its own row. The step itself
//! emits no event: the plan learns the outcome from the aggregate result,
//! which is also what trips a parallel plan's `fail_fast` — a failing cell
//! under `strategy.fail_fast = false` must not stop its sibling cells.

use super::abort::AbortSignal;
use super::exec::{exec_command_raw, exec_standalone, ExecEnv, ExecTaskCtx};
use super::parallel::{compute_channel_capacity, resolve_event_budget, resolve_max_parallel};
use super::{RunnerEvent, StepResult};
use ops_core::config::{CommandId, ExecCommandSpec};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// One cell ready to run: its step id (also its row label), its values for
/// failure messages, and its substituted spec.
struct Cell {
    id: CommandId,
    values: String,
    spec: Arc<ExecCommandSpec>,
}

/// A matrix step, expanded and ready to run.
pub(super) struct MatrixRun {
    id: CommandId,
    cells: Vec<Cell>,
    max_parallel: usize,
    fail_fast: bool,
}

impl MatrixRun {
    /// Expand `spec` into its cells, or `None` when it has no strategy.
    ///
    /// # Errors
    ///
    /// The matrix error message, when the strategy is malformed. The loader
    /// validates every matrix, so this only fires for a `Config` built
    /// outside `load_config_at`.
    pub(super) fn prepare(id: &str, spec: &ExecCommandSpec) -> Option<Result<Self, String>> {
        let strategy = spec.strategy.as_ref()?;
        let cells = match spec.matrix_cells(id) {
            Ok(cells) => cells,
            Err(e) => return Some(Err(format!("{e:#}"))),
        };
        let cells: Vec<Cell> = cells
            .into_iter()
            .map(|c| Cell {
                id: CommandId::from(c.id),
                values: c.cell.describe(),
                spec: Arc::new(c.spec),
            })
            .collect();
        // Unset means "all at once", still under the runner-wide cap.
        let max_parallel = strategy
            .max_parallel
            .unwrap_or_else(resolve_max_parallel)
            .clamp(1, cells.len().max(1));
        Some(Ok(Self {
            id: CommandId::from(id),
            cells,
            max_parallel,
            fail_fast: strategy.fail_fast,
        }))
    }

    /// The cell ids, in cell order: the rows this step renders as.
    pub(super) fn cell_ids(&self) -> impl Iterator<Item = &CommandId> {
        self.cells.iter().map(|c| &c.id)
    }
}

/// How a cell ended, as far as the matrix step can tell.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellState {
    /// No terminal event yet (queued, running, or cancelled mid-run).
    Pending,
    Succeeded,
    Failed,
    Skipped,
    /// The cell's task panicked: it never sent a terminal event.
    Panicked,
}

/// Run every cell of `run`, forwarding their events to `tx`, and fold the
/// outcome into one [`StepResult`] for the matrix step.
///
/// `outer_abort` is the enclosing parallel plan's signal: a matrix that
/// starts after it tripped skips every cell, like `exec_standalone` skips a
/// plain step. Once running, the enclosing plan stops the matrix by
/// aborting its task, which drops the cell tasks with it.
pub(super) async fn run_matrix(
    run: MatrixRun,
    env: ExecEnv,
    tx: mpsc::Sender<RunnerEvent>,
    outer_abort: Option<Arc<AbortSignal>>,
) -> StepResult {
    let start = Instant::now();
    let MatrixRun {
        id,
        cells,
        max_parallel,
        fail_fast,
    } = run;
    let mut states = vec![CellState::Pending; cells.len()];
    if outer_abort.is_some_and(|a| a.is_set()) {
        finish_pending(&cells, &mut states, &tx).await;
        return StepResult::cancelled(id);
    }

    let capacity = compute_channel_capacity(cells.len(), max_parallel, resolve_event_budget());
    let (cell_tx, mut cell_rx) = mpsc::channel(capacity);
    // The cells' own abort: tripped by `strategy.fail_fast`, never by the
    // enclosing plan.
    let abort = Arc::new(AbortSignal::new());
    let semaphore = Arc::new(tokio::sync::Semaphore::new(max_parallel));
    let mut join_set = tokio::task::JoinSet::new();
    let index_by_id: HashMap<CommandId, usize> = cells
        .iter()
        .enumerate()
        .map(|(i, cell)| (cell.id.clone(), i))
        .collect();
    let mut index_by_task: HashMap<tokio::task::Id, usize> = HashMap::with_capacity(cells.len());
    // Cells launch from this loop, in cell order, each once a permit is
    // free: spawning them all up front would let the runtime's polling
    // order decide which cell takes a permit first. The driver's sender is
    // dropped once launching ends, so the channel closes with the last cell.
    let mut launcher = Some(cell_tx);
    let mut next = 0usize;

    let trip = |abort: &AbortSignal, join_set: &mut tokio::task::JoinSet<StepResult>| {
        if fail_fast && !abort.is_set() {
            tracing::debug!(matrix = ?id.as_str(), "matrix fail_fast: cancelling remaining cells");
            abort.set();
            join_set.abort_all();
        }
    };
    let mut rx_open = true;
    loop {
        if next >= cells.len() || abort.is_set() {
            launcher = None;
        }
        if launcher.is_none() && !rx_open && join_set.is_empty() {
            break;
        }
        tokio::select! {
            permit = Arc::clone(&semaphore).acquire_owned(), if launcher.is_some() => {
                // The semaphore is never closed; treat a closed one as "stop
                // launching" and let the pending cells render as skipped.
                let (Ok(permit), Some(cell_tx), Some(cell)) = (permit, &launcher, cells.get(next))
                else {
                    launcher = None;
                    continue;
                };
                let ctx = ExecTaskCtx::new(env.clone(), cell_tx.clone(), Arc::clone(&abort));
                let (cell_id, spec) = (cell.id.clone(), Arc::clone(&cell.spec));
                let handle = join_set.spawn(async move {
                    let _permit = permit;
                    exec_standalone(cell_id, spec, ctx).await
                });
                index_by_task.insert(handle.id(), next);
                // Bounded by `cells.len()`, so exactly `+= 1`.
                next = next.saturating_add(1);
            }
            ev = cell_rx.recv(), if rx_open => {
                let Some(mut ev) = ev else {
                    rx_open = false;
                    continue;
                };
                if let Some((cell_id, state)) = label_cell_event(&mut ev) {
                    if let Some(slot) = index_by_id.get(&cell_id).and_then(|i| states.get_mut(*i)) {
                        *slot = state;
                    }
                    if state == CellState::Failed {
                        trip(&abort, &mut join_set);
                    }
                }
                // Awaited in the arm body, not raced in a branch, so an
                // event is never dropped by losing the select (ASYNC-16).
                if tx.send(ev).await.is_err() {
                    tracing::debug!(matrix = ?id.as_str(), "outer event channel closed");
                }
            }
            joined = join_set.join_next_with_id(), if !join_set.is_empty() => {
                match joined {
                    Some(Ok((_, result))) if !result.success => trip(&abort, &mut join_set),
                    Some(Err(e)) if !e.is_cancelled() => {
                        tracing::debug!(matrix = ?id.as_str(), error = %e, "matrix cell panicked");
                        if let Some(slot) = index_by_task.get(&e.id()).and_then(|i| states.get_mut(*i)) {
                            *slot = CellState::Panicked;
                        }
                        trip(&abort, &mut join_set);
                    }
                    _ => {}
                }
            }
        }
    }
    finish_pending(&cells, &mut states, &tx).await;
    aggregate(id, &cells, &states, start.elapsed())
}

/// Give every cell that never reported an end a terminal row: skipped when
/// it was cancelled or never started, failed when its task panicked.
async fn finish_pending(cells: &[Cell], states: &mut [CellState], tx: &mpsc::Sender<RunnerEvent>) {
    for (cell, state) in cells.iter().zip(states.iter_mut()) {
        let display_cmd = Some(cell.id.to_string());
        let ev = match *state {
            CellState::Pending => {
                *state = CellState::Skipped;
                RunnerEvent::StepSkipped {
                    id: cell.id.clone(),
                    display_cmd,
                }
            }
            CellState::Panicked => RunnerEvent::StepFailed {
                id: cell.id.clone(),
                duration_secs: 0.0,
                message: "task panicked".to_string(),
                display_cmd,
            },
            CellState::Succeeded | CellState::Failed | CellState::Skipped => continue,
        };
        // A closed channel means the display is gone; the aggregate result
        // still reports every cell.
        let _ = tx.send(ev).await;
    }
}

/// Relabel a cell event with the cell id and report the terminal state it
/// records, if any.
fn label_cell_event(ev: &mut RunnerEvent) -> Option<(CommandId, CellState)> {
    let (id, display_cmd, state) = match ev {
        RunnerEvent::StepStarted { id, display_cmd } => (id, display_cmd, CellState::Pending),
        RunnerEvent::StepFinished {
            id, display_cmd, ..
        } => (id, display_cmd, CellState::Succeeded),
        RunnerEvent::StepFailed {
            id, display_cmd, ..
        } => (id, display_cmd, CellState::Failed),
        RunnerEvent::StepSkipped { id, display_cmd } => (id, display_cmd, CellState::Skipped),
        _ => return None,
    };
    *display_cmd = Some(id.to_string());
    Some((id.clone(), state))
}

/// Fold the cells' outcomes into the matrix step's result: success only when
/// every cell succeeded; otherwise the message names every failed cell by
/// its values, and counts the cells `fail_fast` skipped.
fn aggregate(
    id: CommandId,
    cells: &[Cell],
    states: &[CellState],
    duration: Duration,
) -> StepResult {
    let failed: Vec<String> = cells
        .iter()
        .zip(states)
        .filter(|(_, s)| matches!(s, CellState::Failed | CellState::Panicked))
        .map(|(c, _)| format!("[{}]", c.values))
        .collect();
    let skipped = states.iter().filter(|s| **s == CellState::Skipped).count();
    if failed.is_empty() && skipped == 0 {
        return StepResult::success_with_stdout(id, duration, String::new());
    }
    let skipped_note = if skipped > 0 {
        format!("; {skipped} skipped")
    } else {
        String::new()
    };
    let message = format!(
        "{} of {} cells failed: {}{skipped_note}",
        failed.len(),
        cells.len(),
        failed.join(", ")
    );
    StepResult::failure(id, duration, message)
}

/// `--raw` mode: run the cells one after another with inherited stdio (raw
/// runs everything sequentially), honouring `strategy.fail_fast`.
pub(super) async fn run_matrix_raw(run: MatrixRun, env: &ExecEnv) -> StepResult {
    let start = Instant::now();
    let mut states = vec![CellState::Skipped; run.cells.len()];
    for (cell, state) in run.cells.iter().zip(states.iter_mut()) {
        let result = exec_command_raw(cell.id.as_str(), &cell.spec, env).await;
        *state = if result.success {
            CellState::Succeeded
        } else {
            CellState::Failed
        };
        if !result.success && run.fail_fast {
            break;
        }
    }
    aggregate(run.id, &run.cells, &states, start.elapsed())
}
