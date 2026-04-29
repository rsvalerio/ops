//! Tests for parallel plan execution and exec_standalone.

use super::*;

#[tokio::test(flavor = "multi_thread")]
async fn run_plan_parallel_success() {
    let mut commands = HashMap::new();
    let echo_spec = CommandSpec::Exec(echo_cmd("a"));
    commands.insert("e1".to_string(), echo_spec.clone());
    commands.insert("e2".to_string(), echo_spec);
    let runner = test_runner(commands);
    let mut events = Vec::new();
    let results = runner
        .run_plan_parallel(&["e1".into(), "e2".into()], true, &mut |e| events.push(e))
        .await;
    assert!(results.iter().all(|r| r.success));
    assert!(events
        .iter()
        .any(|e| matches!(e, RunnerEvent::PlanStarted { .. })));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, RunnerEvent::StepFinished { .. }))
            .count(),
        2
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, RunnerEvent::RunFinished { success: true, .. })));
}

#[tokio::test(flavor = "multi_thread")]
async fn run_plan_parallel_verify_event_content() {
    let mut commands = HashMap::new();
    commands.insert(
        "echo_a".to_string(),
        CommandSpec::Exec(echo_cmd("test_output")),
    );
    let runner = test_runner(commands);
    let mut events = Vec::new();
    let results = runner
        .run_plan_parallel(&["echo_a".into()], false, &mut |e| events.push(e))
        .await;

    assert!(
        results.iter().all(|r| r.success),
        "all results should succeed"
    );

    let plan_started: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, RunnerEvent::PlanStarted { .. }))
        .collect();
    assert_eq!(plan_started.len(), 1, "should have exactly one PlanStarted");
    if let RunnerEvent::PlanStarted { command_ids } = &plan_started[0] {
        assert_eq!(
            command_ids,
            &vec!["echo_a"],
            "PlanStarted should contain correct command_ids"
        );
    }

    let step_finished: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, RunnerEvent::StepFinished { .. }))
        .collect();
    assert_eq!(
        step_finished.len(),
        1,
        "should have exactly one StepFinished"
    );
    if let RunnerEvent::StepFinished {
        id, duration_secs, ..
    } = &step_finished[0]
    {
        assert_eq!(id, "echo_a", "StepFinished should have correct id");
        assert!(duration_secs > &0.0, "duration_secs should be positive");
    }

    let run_finished: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, RunnerEvent::RunFinished { .. }))
        .collect();
    assert_eq!(run_finished.len(), 1, "should have exactly one RunFinished");
    if let RunnerEvent::RunFinished { success, .. } = &run_finished[0] {
        assert!(success, "RunFinished should indicate success");
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn run_plan_parallel_fail_fast_emits_failure() {
    let mut commands = HashMap::new();
    commands.insert("ok".to_string(), CommandSpec::Exec(true_cmd()));
    commands.insert("fail".to_string(), CommandSpec::Exec(false_cmd()));
    let runner = test_runner(commands);
    let mut events = Vec::new();
    let results = runner
        .run_plan_parallel(&["ok".into(), "fail".into()], true, &mut |e| events.push(e))
        .await;
    assert!(!results.iter().all(|r| r.success), "run should fail");
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RunnerEvent::StepFailed { .. })),
        "should emit StepFailed"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RunnerEvent::RunFinished { success: false, .. })),
        "should emit RunFinished with success=false"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn run_plan_parallel_no_fail_fast() {
    let mut commands = HashMap::new();
    commands.insert("ok".to_string(), CommandSpec::Exec(true_cmd()));
    commands.insert("fail".to_string(), CommandSpec::Exec(false_cmd()));
    let runner = test_runner(commands);
    let mut events = Vec::new();
    let results = runner
        .run_plan_parallel(&["ok".into(), "fail".into()], false, &mut |e| {
            events.push(e)
        })
        .await;
    assert!(!results.iter().all(|r| r.success));
    let finished = events
        .iter()
        .filter(|e| matches!(e, RunnerEvent::StepFinished { .. }))
        .count();
    let failed = events
        .iter()
        .filter(|e| matches!(e, RunnerEvent::StepFailed { .. }))
        .count();
    assert_eq!(
        finished + failed,
        2,
        "both steps should complete (one ok, one fail)"
    );
    assert!(events
        .iter()
        .any(|e| matches!(e, RunnerEvent::RunFinished { success: false, .. })));
}

#[tokio::test]
async fn run_parallel_composite() {
    let mut commands = HashMap::new();
    commands.insert("a".to_string(), CommandSpec::Exec(echo_cmd("a")));
    commands.insert("b".to_string(), CommandSpec::Exec(echo_cmd("b")));
    commands.insert(
        "par".to_string(),
        CommandSpec::Composite(parallel_cmd(&["a", "b"])),
    );
    let runner = test_runner(commands);
    let mut events = Vec::new();
    let results = runner
        .run("par", &mut |e| events.push(e))
        .await
        .expect("run should not error");
    assert!(results.iter().all(|r| r.success));
    assert_eq!(
        events
            .iter()
            .filter(|e| matches!(e, RunnerEvent::StepFinished { .. }))
            .count(),
        2
    );
}

/// TASK-0328: exec_standalone routes terminal events past the bounded local
/// buffer via the awaited outer `tx.send`, specifically so the display can
/// never orphan a progress bar when a noisy command floods the 256-slot
/// LOCAL_BUF.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn exec_standalone_delivers_terminal_event_under_high_volume_load() {
    let (tx, mut rx) = mpsc::channel(8);
    let abort = Arc::new(AbortSignal::new());
    let spec = exec_spec(
        "sh",
        &["-c", "for i in $(seq 1 500); do echo line_$i; done"],
    );
    let exec_handle = tokio::spawn(exec_standalone(
        "buffer_full".into(),
        spec,
        Arc::new(PathBuf::from(".")),
        Arc::new(test_vars()),
        tx,
        abort,
    ));

    let mut events = Vec::new();
    while let Some(ev) = rx.recv().await {
        events.push(ev);
    }
    let result = exec_handle.await.expect("exec_standalone task panicked");

    assert!(result.success, "high-volume command should succeed");
    assert!(
        events.len() > 1,
        "should have observed multiple events, got {}",
        events.len()
    );

    let terminal_idx = events
        .iter()
        .rposition(|e| {
            matches!(
                e,
                RunnerEvent::StepFinished { .. }
                    | RunnerEvent::StepFailed { .. }
                    | RunnerEvent::StepSkipped { .. }
            )
        })
        .expect("terminal event must be delivered on outer rx");
    assert_eq!(
        terminal_idx,
        events.len() - 1,
        "terminal event must arrive after every forwarded StepOutput (no out-of-order delivery)"
    );
}

/// TASK-0335 #2: aborting the parent of `exec_standalone` must not leave the
/// forwarder task pending in the runtime.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn exec_standalone_aborts_forwarder_on_outer_cancellation() {
    use tokio::time::timeout;

    let (tx, mut rx) = mpsc::channel::<RunnerEvent>(1);
    let abort = Arc::new(AbortSignal::new());
    let spec = exec_spec(
        "sh",
        &[
            "-c",
            "for i in $(seq 1 1000); do echo line_$i; done; sleep 5",
        ],
    );

    let handle = tokio::spawn(exec_standalone(
        "leak_test".into(),
        spec,
        Arc::new(PathBuf::from(".")),
        Arc::new(test_vars()),
        tx,
        abort,
    ));

    let _ = timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("first event should arrive in time")
        .expect("forwarder should deliver at least one event");

    handle.abort();
    let _ = handle.await;

    let outcome = timeout(std::time::Duration::from_secs(5), rx.recv())
        .await
        .expect("rx.recv must resolve — forwarder must have dropped its `outer` clone");
    assert!(
        outcome.is_none(),
        "expected channel close (forwarder aborted, all senders dropped); got {:?}",
        outcome
    );
}

/// CONC-7 / TASK-0457: a chatty producer that bursts past the 256-slot
/// per-task buffer must either deliver every line or surface a
/// `StepOutputDropped { id, dropped_count }` so the display can render
/// "(N output lines dropped under load)" — silent drops are the bug
/// this regression test pins. Stalls the receiver to make drops likely
/// in CI without depending on timing.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn exec_standalone_emits_step_output_dropped_under_burst() {
    let (tx, mut rx) = mpsc::channel::<RunnerEvent>(8);
    let abort = Arc::new(AbortSignal::new());
    let spec = exec_spec(
        "sh",
        &["-c", "for i in $(seq 1 1500); do echo line_$i; done"],
    );

    let handle = tokio::spawn(exec_standalone(
        "burst".into(),
        spec,
        Arc::new(PathBuf::from(".")),
        Arc::new(test_vars()),
        tx,
        Arc::clone(&abort),
    ));

    // Pause the receiver briefly to make backpressure likely while the
    // producer races ahead, then drain.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;

    let mut events = Vec::new();
    while let Some(ev) = rx.recv().await {
        events.push(ev);
    }
    let _ = handle.await;
    // Re-establish the no-leak invariant so a future change cannot
    // accidentally rely on lazy aborts.
    assert!(
        !abort.is_set(),
        "abort flag must not have been tripped by the test producer"
    );

    let stdout_lines = events
        .iter()
        .filter(|e| matches!(e, RunnerEvent::StepOutput { stderr: false, .. }))
        .count();
    let dropped: u64 = events
        .iter()
        .filter_map(|e| match e {
            RunnerEvent::StepOutputDropped { id, dropped_count } if id.as_str() == "burst" => {
                Some(*dropped_count)
            }
            _ => None,
        })
        .sum();
    let total = stdout_lines as u64 + dropped;
    assert_eq!(
        total, 1500,
        "every produced line must either be delivered or counted as dropped — got {stdout_lines} delivered + {dropped} dropped"
    );
    if dropped > 0 {
        // If anything was dropped, the explicit event must be present.
        assert!(
            events
                .iter()
                .any(|e| matches!(e, RunnerEvent::StepOutputDropped { .. })),
            "drops must be surfaced via StepOutputDropped"
        );
    }
}

/// CONC-9 / TASK-0459: when the outer mpsc receiver stalls (display pump
/// hung) and the abort flag is tripped (a sibling fail_fast'd),
/// exec_standalone must abandon its terminal-event send instead of
/// blocking on a full outer channel. Previously the task would hang
/// indefinitely on `tx.send`, defeating fail_fast's promise.
#[cfg(unix)]
#[tokio::test(flavor = "multi_thread")]
async fn exec_standalone_terminal_send_aborts_on_full_outer_channel() {
    use tokio::time::timeout;

    let (tx, _rx) = mpsc::channel::<RunnerEvent>(1);
    let abort = Arc::new(AbortSignal::new());
    let spec = exec_spec("sh", &["-c", "echo line; echo line; exit 0"]);

    // Fill the outer channel before the task can deliver a terminal event
    // by sending a placeholder. Capacity is 1 so the next send blocks.
    tx.send(RunnerEvent::PlanStarted {
        command_ids: vec![],
    })
    .await
    .unwrap();

    let abort_clone = Arc::clone(&abort);
    let handle = tokio::spawn(exec_standalone(
        "stuck".into(),
        spec,
        Arc::new(PathBuf::from(".")),
        Arc::new(test_vars()),
        tx,
        abort_clone,
    ));

    // Give the task a moment to reach the terminal-send.
    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
    // Trip fail_fast.
    abort.set();

    // The task must complete promptly even though the outer channel is
    // still full and the receiver is parked. Tight 1s budget so a
    // regression hangs the test.
    let outcome = timeout(std::time::Duration::from_secs(1), handle)
        .await
        .expect("exec_standalone must abandon terminal send under abort, not hang");
    let _ = outcome;
}

#[tokio::test(flavor = "multi_thread")]
async fn exec_standalone_skips_when_abort_set() {
    let (tx, mut rx) = mpsc::channel(8);
    let abort = {
        let s = Arc::new(AbortSignal::new());
        s.set();
        s
    };
    let spec = echo_cmd("should not run");
    let result = exec_standalone(
        "skipped".into(),
        spec,
        Arc::new(PathBuf::from(".")),
        Arc::new(test_vars()),
        tx,
        abort,
    )
    .await;
    // TASK-0408: cancellation (abort flag set on entry) is now success=false.
    assert!(!result.success);
    assert_eq!(result.duration, Duration::ZERO);
    let event = rx.recv().await.expect("should receive one event");
    assert!(
        matches!(event, RunnerEvent::StepSkipped { .. }),
        "expected StepSkipped, got {:?}",
        event
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn run_plan_parallel_resolution_failure() {
    let mut commands = HashMap::new();
    commands.insert(
        "comp".to_string(),
        CommandSpec::Composite(composite_cmd(&["a"])),
    );
    let runner = test_runner(commands);
    let mut events = Vec::new();
    let results = runner
        .run_plan_parallel(&["comp".into()], true, &mut |e| events.push(e))
        .await;
    assert!(!results.iter().all(|r| r.success), "should fail");
    assert!(events
        .iter()
        .any(|e| matches!(e, RunnerEvent::RunFinished { success: false, .. })));
}

/// TQ-013: Verify parallel execution actually runs commands concurrently.
#[cfg(unix)]
mod parallel_timing_tests {
    use super::*;

    fn rendezvous_cmd(mine: &std::path::Path, theirs: &std::path::Path) -> ExecCommandSpec {
        let script = format!(
            "touch {mine}; for i in $(seq 1 50); do [ -e {theirs} ] && exit 0; sleep 0.1; done; exit 1",
            mine = shell_escape(mine),
            theirs = shell_escape(theirs),
        );
        exec_spec("sh", &["-c", &script])
    }

    fn shell_escape(p: &std::path::Path) -> String {
        format!("'{}'", p.to_str().unwrap().replace('\'', "'\\''"))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn run_plan_parallel_executes_concurrently() {
        let dir = tempfile::tempdir().unwrap();
        let marker_a = dir.path().join("a");
        let marker_b = dir.path().join("b");

        let mut commands = HashMap::new();
        commands.insert(
            "rdv_a".to_string(),
            CommandSpec::Exec(rendezvous_cmd(&marker_a, &marker_b)),
        );
        commands.insert(
            "rdv_b".to_string(),
            CommandSpec::Exec(rendezvous_cmd(&marker_b, &marker_a)),
        );
        let runner = test_runner(commands);
        let mut events = Vec::new();
        let results = runner
            .run_plan_parallel(&["rdv_a".into(), "rdv_b".into()], true, &mut |e| {
                events.push(e)
            })
            .await;

        assert!(
            results.iter().all(|r| r.success),
            "both commands must rendezvous — failure proves they did not run concurrently"
        );
    }
}

mod parallel_failure_tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn run_plan_parallel_all_fail() {
        let mut commands = HashMap::new();
        commands.insert("fail1".to_string(), CommandSpec::Exec(false_cmd()));
        commands.insert("fail2".to_string(), CommandSpec::Exec(false_cmd()));
        let runner = test_runner(commands);
        let mut events = Vec::new();
        let results = runner
            .run_plan_parallel(&["fail1".into(), "fail2".into()], false, &mut |e| {
                events.push(e)
            })
            .await;

        assert!(
            results.iter().all(|r| !r.success),
            "all results should be failures"
        );
        assert_eq!(results.len(), 2, "both commands should have results");

        let failed_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, RunnerEvent::StepFailed { .. }))
            .collect();
        assert_eq!(
            failed_events.len(),
            2,
            "both failures should emit StepFailed"
        );

        assert!(
            events
                .iter()
                .any(|e| matches!(e, RunnerEvent::RunFinished { success: false, .. })),
            "should emit RunFinished with success=false"
        );
    }
}
