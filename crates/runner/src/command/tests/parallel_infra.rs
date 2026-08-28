//! Tests for the parallel-execution plumbing (spawn / collect / `handle_events`).

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn spawn_parallel_tasks_creates_correct_count() {
    let steps: Vec<(CommandId, _)> = vec![
        ("cmd1".into(), echo_cmd("a")),
        ("cmd2".into(), echo_cmd("b")),
        ("cmd3".into(), echo_cmd("c")),
    ];
    let (rx, _abort, join_set, id_map) = CommandRunner::spawn_parallel_tasks(
        steps,
        &Arc::new(PathBuf::from(".")),
        &Arc::new(test_vars()),
        crate::command::CwdEscapePolicy::WarnAndAllow,
        &Arc::new(WorkspaceCanonicalCache::new()),
    );
    drop(rx);
    let results = CommandRunner::collect_join_results(join_set, &id_map).await;
    assert_eq!(results.len(), 3);
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_parallel_events_receives_all() {
    let (tx, rx) = mpsc::channel(8);
    let abort = Arc::new(AbortSignal::new());

    tx.try_send(RunnerEvent::StepStarted {
        id: "a".into(),
        display_cmd: None,
    })
    .unwrap();
    tx.try_send(RunnerEvent::StepFinished {
        id: "a".into(),
        duration_secs: 0.1,
        display_cmd: None,
    })
    .unwrap();
    drop(tx);

    let mut events = Vec::new();
    CommandRunner::handle_parallel_events(rx, false, abort, &mut |e| events.push(e)).await;

    assert_eq!(events.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_parallel_events_sets_abort_on_fail_fast() {
    let (tx, rx) = mpsc::channel(8);
    let abort = Arc::new(AbortSignal::new());

    tx.try_send(RunnerEvent::StepFailed {
        id: "fail".into(),
        duration_secs: 0.1,
        message: "error".into(),
        display_cmd: None,
    })
    .unwrap();
    drop(tx);

    CommandRunner::handle_parallel_events(rx, true, Arc::clone(&abort), &mut |_| {}).await;

    assert!(
        abort.is_set(),
        "abort should be set on failure with fail_fast=true"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn handle_parallel_events_no_abort_without_fail_fast() {
    let (tx, rx) = mpsc::channel(8);
    let abort = Arc::new(AbortSignal::new());

    tx.try_send(RunnerEvent::StepFailed {
        id: "fail".into(),
        duration_secs: 0.1,
        message: "error".into(),
        display_cmd: None,
    })
    .unwrap();
    drop(tx);

    CommandRunner::handle_parallel_events(rx, false, Arc::clone(&abort), &mut |_| {}).await;

    assert!(!abort.is_set(), "abort should NOT be set without fail_fast");
}

/// CONC-6 / TASK-1177: a panicked sibling under `fail_fast` must trip abort
/// at the same point a `StepFailed` event would. Pre-fix the abort signal
/// only fired on `RunnerEvent::StepFailed`, so a task that panicked rather
/// than emitting a failure event kept its siblings running until the
/// channel drained naturally — defeating `fail_fast` for the panic path.
///
/// Spawn one task that panics immediately and one that holds an
/// `AbortSignal::cancelled()` future the test checks; assert that the
/// signal is set before the sleeping sibling's nominal duration elapses.
#[tokio::test(flavor = "multi_thread")]
async fn fail_fast_aborts_siblings_when_a_task_panics() {
    let (tx, rx) = mpsc::channel(8);
    let abort = Arc::new(AbortSignal::new());
    let mut join_set: tokio::task::JoinSet<StepResult> = tokio::task::JoinSet::new();

    // Task A panics immediately. No StepFailed event will be emitted; the
    // panic surfaces only via JoinSet.
    join_set.spawn(async {
        panic!("simulated task panic");
    });

    // Task B observes the abort signal. Wait up to 5s; if abort fires
    // earlier the task returns early — that is the contract under test.
    let abort_b = Arc::clone(&abort);
    join_set.spawn(async move {
        tokio::select! {
            () = abort_b.cancelled() => {
                StepResult::cancelled(CommandId::from("b"))
            }
            () = tokio::time::sleep(std::time::Duration::from_secs(5)) => {
                StepResult::success("b", Duration::from_secs(5))
            }
        }
    });

    // Drop tx so the events loop's rx-side closes once both tasks are
    // accounted for; the loop continues draining the JoinSet until empty.
    drop(tx);

    let mut harvested: Vec<(tokio::task::Id, Result<StepResult, tokio::task::JoinError>)> =
        Vec::new();
    CommandRunner::handle_parallel_events_with_cancel_inner(
        rx,
        true, // fail_fast
        Arc::clone(&abort),
        &mut join_set,
        &mut |_| {},
        &mut harvested,
    )
    .await;

    assert!(
        abort.is_set(),
        "abort signal must be tripped when a sibling task panics under fail_fast"
    );
    // TEST-15 / TASK-1664: the `elapsed < 4s` assertion that used to sit here
    // is deleted as redundant load-sensitive surface. If abort never fired,
    // task B would fall through to its 5s sleep and return
    // `StepResult::success("b", 5s)` — which the harvest assertion below
    // already rejects, because it requires the sibling to be cancelled or an
    // aborted `Err`. The timing assertion could therefore only ever fail
    // alongside an assertion that states the contract directly.
    // Two harvested entries: the panic (Err) and the cancelled sibling
    // (Ok with success=false from `StepResult::cancelled`, OR Err with
    // is_cancelled — abort_all aborts the JoinSet which yields Err).
    assert_eq!(
        harvested.len(),
        2,
        "both tasks must have been harvested before the loop exited"
    );
}

/// CONC-3 / TASK-1921: the panic trigger must fire while a sibling is still
/// flooding `StepOutput`, not only once the flood stops.
///
/// The pre-fix loop polled its two `select!` arms `biased`, top-to-bottom,
/// so the `JoinSet` arm was reached only on an iteration where `rx.recv()`
/// was *pending*. This test removes that window on purpose: the flooding
/// task never returns on its own, and the event callback sleeps a
/// millisecond per event, so the producer always refills the channel before
/// the loop polls it again and `rx.recv()` is ready every single time. The
/// panicked sibling is then the only way out of the loop — under `biased`
/// it is never observed and this test hangs to its timeout, which is
/// precisely the `fail_fast` promise the CONC-6 / TASK-1177 contract makes.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn fail_fast_trips_on_panic_while_a_sibling_floods_output() {
    let (tx, rx) = mpsc::channel(64);
    let abort = Arc::new(AbortSignal::new());
    let mut join_set: tokio::task::JoinSet<StepResult> = tokio::task::JoinSet::new();

    // Task A floods the event channel and never completes on its own; only
    // `abort_all` (or a closed channel) can stop it.
    let flood_tx = tx.clone();
    join_set.spawn(async move {
        let buf: std::sync::Arc<str> = std::sync::Arc::from("noisy output line");
        loop {
            let ev = RunnerEvent::StepOutput {
                id: "flooder".into(),
                line: crate::command::events::OutputLine::whole(std::sync::Arc::clone(&buf)),
                stderr: false,
            };
            if flood_tx.send(ev).await.is_err() {
                break;
            }
        }
        StepResult::success("flooder", Duration::ZERO)
    });

    // Task B panics once the flood is underway. It emits no StepFailed
    // event, so the JoinSet arm is the only thing that can notice it.
    join_set.spawn(async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        panic!("simulated task panic under flood");
    });

    // Only the flooder holds a sender now, so the channel closes exactly
    // when the flooder is aborted.
    drop(tx);

    let mut seen = 0_usize;
    let mut harvested: Vec<(tokio::task::Id, Result<StepResult, tokio::task::JoinError>)> =
        Vec::new();
    tokio::time::timeout(
        Duration::from_secs(20),
        CommandRunner::handle_parallel_events_with_cancel_inner(
            rx,
            true, // fail_fast
            Arc::clone(&abort),
            &mut join_set,
            &mut |_| {
                seen += 1;
                // Blocking on purpose: it keeps the channel saturated so
                // `rx.recv()` is ready on every poll of the loop.
                std::thread::sleep(Duration::from_millis(1));
            },
            &mut harvested,
        ),
    )
    .await
    .expect("panicked task must trip fail_fast while the sibling is still flooding");

    assert!(
        abort.is_set(),
        "abort must be tripped by the panicked task despite the ongoing flood"
    );
    assert_eq!(harvested.len(), 2, "both tasks must have been harvested");
    // The flood is unbounded; a bounded event count is the observable proof
    // that the abort landed mid-flood rather than after it drained.
    assert!(
        seen < 2_000,
        "abort should land while the flood is in flight, saw {seen} events first"
    );
}

/// TASK-0334: panic payloads from `JoinError` must not surface verbatim in
/// `StepResult.message`.
#[tokio::test(flavor = "multi_thread")]
async fn collect_join_results_redacts_panic_payload() {
    let mut join_set = tokio::task::JoinSet::new();
    let panic_handle =
        join_set.spawn(async { panic!("/Users/secret/home/.aws/credentials missing") });
    let mut id_map: std::collections::HashMap<tokio::task::Id, CommandId> =
        std::collections::HashMap::new();
    id_map.insert(panic_handle.id(), CommandId::from("real-panicker"));
    join_set.spawn(async { StepResult::success("ok", Duration::from_millis(10)) });

    let results = CommandRunner::collect_join_results(join_set, &id_map).await;

    assert_eq!(results.len(), 2);
    // READ-5 / TASK-0767: panicked task carries the originating CommandId,
    // not the previous "<panicked>" sentinel.
    let panic_result = results.iter().find(|r| r.id == "real-panicker").unwrap();
    assert!(!panic_result.success);
    let msg = panic_result.message.as_ref().unwrap();
    assert!(
        !msg.contains("/Users/"),
        "redacted message must not leak the panic payload path, got: {msg}"
    );
    assert!(
        !msg.contains("credentials"),
        "redacted message must not leak attacker-controlled payload text, got: {msg}"
    );
    assert!(
        msg.contains("panicked"),
        "message should still convey that the task panicked, got: {msg}"
    );
}
