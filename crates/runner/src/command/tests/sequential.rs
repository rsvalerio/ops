//! Tests for sequential plan execution.

use super::*;

#[tokio::test]
async fn run_plan_unknown_command_emits_failure() {
    let runner = test_runner(HashMap::new());
    let mut events = Vec::new();
    let results = runner
        .run_plan(&["nonexistent".into()], true, &mut |e| events.push(e))
        .await;
    assert!(!results.iter().all(|r| r.success));
    let failed = events
        .iter()
        .find(|e| matches!(e, RunnerEvent::StepFailed { .. }));
    assert!(
        failed.is_some(),
        "should emit StepFailed for unknown command"
    );
    if let Some(RunnerEvent::StepFailed { id, message, .. }) = failed {
        // TEST-11: pin the exact error text so a regression that renders
        // the wrong id (or drops the id entirely) is caught. Substring-only
        // `contains("unknown command")` tolerates both of those bugs.
        assert_eq!(id.as_str(), "nonexistent", "failed event must carry the id");
        assert_eq!(
            message, "unknown command: nonexistent",
            "exact failure message mismatch"
        );
    }
}

#[tokio::test]
async fn run_sequential_composite() {
    let mut commands = HashMap::new();
    commands.insert("a".to_string(), CommandSpec::Exec(echo_cmd("a")));
    commands.insert("b".to_string(), CommandSpec::Exec(echo_cmd("b")));
    commands.insert(
        "both".to_string(),
        CommandSpec::Composite(composite_cmd(&["a", "b"])),
    );
    let runner = test_runner(commands);
    let mut events = Vec::new();
    let results = runner
        .run("both", &mut |e| events.push(e))
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
