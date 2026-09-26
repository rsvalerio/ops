//! TASK-2277: matrix commands run once per cell as one plan step.
//!
//! Cells are `sh -c` scripts so a cell's outcome is picked by its value.
//! Overlap is proven with a `mkdir` lock rather than timing: `mkdir` fails
//! when the directory exists, so two cells (or a cell and a sibling step)
//! holding the lock at once make one of them fail, deterministically.
#![cfg(unix)]

use super::*;
use ops_core::config::{CommandId, ExecCommandSpec, Matrix, Strategy};

/// `sh -c <script>` as a matrix command over `n = values`.
fn matrix_cmd(script: &str, values: &[&str], max_parallel: usize, fail_fast: bool) -> CommandSpec {
    let mut spec = exec_spec("sh", &["-c", script]);
    let mut matrix = Matrix::default();
    matrix.axes.insert(
        "n".into(),
        values.iter().map(|v| (*v).to_string()).collect(),
    );
    let mut strategy = Strategy::new(matrix);
    strategy.max_parallel = Some(max_parallel);
    strategy.fail_fast = fail_fast;
    spec.strategy = Some(strategy);
    CommandSpec::Exec(spec)
}

/// A script holding `lock` for a moment; fails when it is already held.
fn locked(lock: &std::path::Path, extra: &str) -> String {
    let lock = lock.display();
    format!("mkdir {lock} || exit 9; sleep 0.2; rmdir {lock}; {extra}")
}

fn ids_where(events: &[RunnerEvent], pick: fn(&RunnerEvent) -> Option<&CommandId>) -> Vec<String> {
    events
        .iter()
        .filter_map(pick)
        .map(ToString::to_string)
        .collect()
}

fn failed(e: &RunnerEvent) -> Option<&CommandId> {
    match e {
        RunnerEvent::StepFailed { id, .. } => Some(id),
        _ => None,
    }
}

fn finished(e: &RunnerEvent) -> Option<&CommandId> {
    match e {
        RunnerEvent::StepFinished { id, .. } => Some(id),
        _ => None,
    }
}

fn skipped(e: &RunnerEvent) -> Option<&CommandId> {
    match e {
        RunnerEvent::StepSkipped { id, .. } => Some(id),
        _ => None,
    }
}

/// AC #1/#5/#6: one aggregate result, one row per cell labelled by its
/// values, and `fail_fast = false` runs every cell and names each failure.
#[tokio::test]
async fn sequential_matrix_runs_every_cell_and_names_each_failure() {
    let mut commands = HashMap::new();
    commands.insert(
        "m".to_string(),
        matrix_cmd(
            "test ${matrix.n} != 2 && test ${matrix.n} != 4",
            &["1", "2", "3", "4"],
            1,
            false,
        ),
    );
    let runner = test_runner(commands);
    let mut events = Vec::new();
    let results = runner
        .run_plan(&["m".into()], true, &mut |e| events.push(e))
        .await;

    assert_eq!(results.len(), 1, "a matrix is one step: {results:?}");
    let result = &results[0];
    assert_eq!(result.id.as_str(), "m");
    assert!(!result.success);
    assert_eq!(
        result.message.as_deref(),
        Some("2 of 4 cells failed: [n=2], [n=4]")
    );

    let Some(RunnerEvent::PlanStarted { command_ids }) = events.first() else {
        panic!("first event must be PlanStarted: {events:?}");
    };
    assert_eq!(
        command_ids,
        &vec!["m [n=1]", "m [n=2]", "m [n=3]", "m [n=4]"],
        "the plan renders one row per cell"
    );
    assert_eq!(ids_where(&events, failed), ["m [n=2]", "m [n=4]"]);
    assert_eq!(ids_where(&events, finished), ["m [n=1]", "m [n=3]"]);
    assert!(
        events.iter().all(|e| match e {
            RunnerEvent::StepStarted { id, display_cmd }
            | RunnerEvent::StepFinished {
                id, display_cmd, ..
            }
            | RunnerEvent::StepFailed {
                id, display_cmd, ..
            } => display_cmd.as_deref() == Some(id.as_str()),
            _ => true,
        }),
        "cell rows are labelled by the cell id: {events:?}"
    );
}

/// AC #5: `strategy.fail_fast = true` stops the remaining cells after the
/// first failure; they render as skipped and the summary counts them.
#[tokio::test]
async fn fail_fast_matrix_skips_the_remaining_cells() {
    let mut commands = HashMap::new();
    commands.insert(
        "m".to_string(),
        matrix_cmd("test ${matrix.n} != 1", &["1", "2", "3"], 1, true),
    );
    let runner = test_runner(commands);
    let mut events = Vec::new();
    let results = runner
        .run_plan(&["m".into()], true, &mut |e| events.push(e))
        .await;
    assert_eq!(
        results[0].message.as_deref(),
        Some("1 of 3 cells failed: [n=1]; 2 skipped")
    );
    assert_eq!(ids_where(&events, skipped), ["m [n=2]", "m [n=3]"]);
    assert!(ids_where(&events, finished).is_empty());
}

/// AC #5: `max_parallel = 1` never overlaps two cells.
#[tokio::test(flavor = "multi_thread")]
async fn max_parallel_one_runs_cells_one_at_a_time() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("lock");
    let mut commands = HashMap::new();
    commands.insert(
        "m".to_string(),
        matrix_cmd(&locked(&lock, "true"), &["1", "2", "3"], 1, false),
    );
    let runner = test_runner(commands);
    let mut events = Vec::new();
    let results = runner
        .run_plan(&["m".into()], true, &mut |e| events.push(e))
        .await;
    assert!(
        results[0].success,
        "cells overlapped: {:?}",
        results[0].message
    );
    let started = ids_where(&events, |e| match e {
        RunnerEvent::StepStarted { id, .. } => Some(id),
        _ => None,
    });
    assert_eq!(
        started,
        ["m [n=1]", "m [n=2]", "m [n=3]"],
        "sequential cells start in cell order, not task-polling order"
    );
}

/// The lock does detect overlap: with room for every cell at once, the
/// same matrix fails.
#[tokio::test(flavor = "multi_thread")]
async fn unbounded_cells_overlap() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("lock");
    let mut commands = HashMap::new();
    commands.insert(
        "m".to_string(),
        matrix_cmd(&locked(&lock, "true"), &["1", "2", "3"], 3, false),
    );
    let runner = test_runner(commands);
    let results = runner.run_plan(&["m".into()], true, &mut |_| {}).await;
    assert!(!results[0].success, "three cells at once must contend");
}

/// AC #4: nested in a `fail_fast = true` parallel group, a
/// `strategy.fail_fast = false` matrix passes expansion (the agreement check
/// is for groups, not matrices), runs every cell although cells fail, and
/// fails the plan as one step.
#[tokio::test(flavor = "multi_thread")]
async fn matrix_in_fail_fast_parallel_group_runs_all_cells() {
    let mut commands = HashMap::new();
    commands.insert(
        "m".to_string(),
        matrix_cmd(
            "sleep 0.1; test ${matrix.n} != 1",
            &["1", "2", "3"],
            1,
            false,
        ),
    );
    commands.insert("other".to_string(), CommandSpec::Exec(echo_cmd("hi")));
    commands.insert(
        "verify".to_string(),
        CommandSpec::Composite(parallel_cmd(&["other", "m"])),
    );
    let runner = test_runner(commands);
    let plan = runner
        .expand_to_plan("verify")
        .expect("no schedule conflict");
    assert_eq!(plan.leaf_ids(), ["other", "m"], "the matrix is one leaf");

    let mut events = Vec::new();
    let results = runner
        .run_plan_tree(&plan, false, &mut |e| events.push(e))
        .await;
    assert_eq!(ids_where(&events, failed), ["m [n=1]"]);
    assert_eq!(
        ids_where(&events, finished)
            .into_iter()
            .filter(|id| id.starts_with('m'))
            .collect::<Vec<_>>(),
        ["m [n=2]", "m [n=3]"],
        "a failing cell must not cancel its sibling cells"
    );
    let m = results.iter().find(|r| r.id.as_str() == "m").unwrap();
    assert!(!m.success);
    assert!(results
        .iter()
        .any(|r| r.id.as_str() == "other" && r.success));
}

/// AC #4: `exclusive` covers the whole matrix — no sibling step overlaps
/// any cell, while the cells themselves may run together.
#[tokio::test(flavor = "multi_thread")]
async fn exclusive_matrix_runs_alone_in_a_parallel_plan() {
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("lock");
    let mut commands = HashMap::new();
    let CommandSpec::Exec(mut m) = matrix_cmd(&locked(&lock, "true"), &["1", "2"], 1, false) else {
        panic!("matrix_cmd builds an exec spec")
    };
    m.exclusive = true;
    commands.insert("m".to_string(), CommandSpec::Exec(m));
    let sibling: ExecCommandSpec = exec_spec("sh", &["-c", &locked(&lock, "true")]);
    commands.insert("a".to_string(), CommandSpec::Exec(sibling.clone()));
    commands.insert("b".to_string(), CommandSpec::Exec(sibling));
    commands.insert(
        "verify".to_string(),
        CommandSpec::Composite(parallel_cmd(&["a", "m", "b"])),
    );
    let runner = test_runner(commands);
    let plan = runner.expand_to_plan("verify").unwrap();
    let results = runner.run_plan_tree(&plan, false, &mut |_| {}).await;
    assert!(
        results.iter().all(|r| r.success),
        "an exclusive matrix must not overlap its siblings: {results:?}"
    );
}

/// AC #5, `--raw`: cells run one after another and the aggregate names
/// every failed cell.
#[tokio::test]
async fn raw_matrix_runs_cells_sequentially() {
    let mut commands = HashMap::new();
    commands.insert(
        "m".to_string(),
        matrix_cmd("test ${matrix.n} = 2", &["1", "2", "3"], 3, false),
    );
    let runner = test_runner(commands);
    let results = runner.run_plan_raw(&["m".into()], true).await;
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].message.as_deref(),
        Some("2 of 3 cells failed: [n=1], [n=3]")
    );
}

/// Inside a parallel batch, cells draw from the batch's `OPS_MAX_PARALLEL`
/// semaphore too: with a shared budget of 1, an otherwise unbounded matrix
/// runs its cells one at a time — and does not deadlock on the permit its
/// own step task gave back.
#[tokio::test(flavor = "multi_thread")]
async fn cells_share_the_enclosing_batch_budget() {
    use crate::command::matrix::{run_matrix, Enclosing, MatrixRun};
    let dir = tempfile::tempdir().unwrap();
    let lock = dir.path().join("lock");
    let CommandSpec::Exec(spec) = matrix_cmd(&locked(&lock, "true"), &["1", "2", "3"], 3, false)
    else {
        panic!("matrix_cmd builds an exec spec")
    };
    let run = MatrixRun::prepare("m", &spec).unwrap().unwrap();
    let (tx, mut rx) = mpsc::channel(64);
    let enclosing = Enclosing {
        abort: Arc::new(AbortSignal::new()),
        budget: Arc::new(tokio::sync::Semaphore::new(1)),
    };
    let result = tokio::time::timeout(
        Duration::from_secs(20),
        run_matrix(run, test_exec_env(), tx, Some(enclosing)),
    )
    .await
    .expect("a shared budget of 1 must not deadlock the matrix");
    while rx.try_recv().is_ok() {}
    assert!(result.success, "cells overlapped: {:?}", result.message);
}

#[test]
fn row_ids_expand_matrix_leaves_only() {
    let mut commands = HashMap::new();
    commands.insert("m".to_string(), matrix_cmd("true", &["a", "b"], 1, true));
    commands.insert("plain".to_string(), CommandSpec::Exec(true_cmd()));
    let runner = test_runner(commands);
    assert_eq!(
        runner.row_ids(&["plain".into(), "m".into()]),
        ["plain", "m [n=a]", "m [n=b]"]
    );
}
