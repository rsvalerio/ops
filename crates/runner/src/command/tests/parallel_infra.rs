//! Tests for the parallel-execution plumbing (spawn / collect / handle_events).

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
        Arc::new(PathBuf::from(".")),
        Arc::new(test_vars()),
        crate::command::CwdEscapePolicy::WarnAndAllow,
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

    let mut events = Vec::new();
    CommandRunner::handle_parallel_events(rx, true, Arc::clone(&abort), &mut |e| events.push(e))
        .await;

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

    let mut events = Vec::new();
    CommandRunner::handle_parallel_events(rx, false, Arc::clone(&abort), &mut |e| events.push(e))
        .await;

    assert!(!abort.is_set(), "abort should NOT be set without fail_fast");
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
