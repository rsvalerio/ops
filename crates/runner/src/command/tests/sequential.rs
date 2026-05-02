//! Tests for sequential plan execution.

use super::*;

/// SEC-14 / TASK-0886: when the runner is configured with
/// `CwdEscapePolicy::Deny` (the policy the hook-triggered entry points
/// install), a command spec with `cwd = "/etc"` (or any escaping path)
/// must be refused at spawn time. Pins the threading from
/// `CommandRunner::cwd_escape_policy` through `exec_command` →
/// `build_command_async` → `apply_escape_policy::Deny` so a future
/// refactor that drops the parameter at any layer reverts to fail-open.
#[tokio::test]
async fn deny_policy_refuses_escaping_cwd_on_hook_path() {
    let mut commands = HashMap::new();
    let spec = exec_spec_with_cwd("true", &[] as &[&str], Some(PathBuf::from("/etc")));
    commands.insert("malicious".to_string(), CommandSpec::Exec(spec));
    let mut runner = test_runner(commands);
    runner.set_cwd_escape_policy(crate::command::CwdEscapePolicy::Deny);

    let mut events = Vec::new();
    let results = runner
        .run_plan(&["malicious".into()], true, &mut |e| events.push(e))
        .await;

    assert_eq!(results.len(), 1);
    assert!(
        !results[0].success,
        "Deny must refuse to spawn when cwd escapes the workspace"
    );
    let msg = results[0]
        .message
        .as_deref()
        .expect("a Deny-refused step carries a message");
    assert!(
        msg.contains("PermissionDenied") || msg.contains("Permission denied"),
        "refusal must surface PermissionDenied, got: {msg}"
    );
}

/// SEC-14: complement to the Deny test above — under `WarnAndAllow`
/// (the default for interactive `ops <cmd>`) the same escaping cwd
/// must NOT be refused at the policy layer. The spawn may still fail
/// (no /etc/true binary on most systems) but the failure shape must
/// not be the SEC-14 PermissionDenied refusal — that is the precise
/// behaviour-change-free guarantee the hook split was designed for.
#[tokio::test]
async fn warn_policy_does_not_refuse_escaping_cwd_on_interactive_path() {
    let mut commands = HashMap::new();
    let spec = exec_spec_with_cwd("true", &[] as &[&str], Some(PathBuf::from("/etc")));
    commands.insert("interactive".to_string(), CommandSpec::Exec(spec));
    let runner = test_runner(commands); // default policy = WarnAndAllow

    let mut events = Vec::new();
    let results = runner
        .run_plan(&["interactive".into()], true, &mut |e| events.push(e))
        .await;

    assert_eq!(results.len(), 1);
    if let Some(msg) = results[0].message.as_deref() {
        assert!(
            !msg.contains("SEC-14"),
            "WarnAndAllow must not surface the SEC-14 refusal, got: {msg}"
        );
    }
}

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
