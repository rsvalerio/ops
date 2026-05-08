//! Tests for raw-mode execution.

use super::*;
use crate::command::exec::exec_command_raw;

#[tokio::test]
async fn exec_command_raw_returns_success_for_true() {
    let spec = true_cmd();
    let cwd = Arc::new(std::env::current_dir().unwrap());
    let vars = Arc::new(test_vars());
    let cache = Arc::new(WorkspaceCanonicalCache::new());
    let result = exec_command_raw(
        "true_cmd",
        &spec,
        &cache,
        &cwd,
        &vars,
        crate::command::CwdEscapePolicy::WarnAndAllow,
    )
    .await;
    assert!(result.success);
    assert!(result.stdout.is_empty(), "raw mode must not capture stdout");
    assert!(result.stderr.is_empty(), "raw mode must not capture stderr");
    assert!(result.message.is_none());
}

#[tokio::test]
async fn exec_command_raw_returns_failure_for_false() {
    let spec = false_cmd();
    let cwd = Arc::new(std::env::current_dir().unwrap());
    let vars = Arc::new(test_vars());
    let cache = Arc::new(WorkspaceCanonicalCache::new());
    let result = exec_command_raw(
        "false_cmd",
        &spec,
        &cache,
        &cwd,
        &vars,
        crate::command::CwdEscapePolicy::WarnAndAllow,
    )
    .await;
    assert!(!result.success);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    assert!(result.message.is_some());
}

#[tokio::test]
async fn run_plan_raw_runs_sequentially_and_collects_results() {
    let mut commands = HashMap::new();
    commands.insert("step_a".to_string(), CommandSpec::Exec(true_cmd()));
    commands.insert("step_b".to_string(), CommandSpec::Exec(true_cmd()));
    let runner = test_runner(commands);

    let results = runner
        .run_plan_raw(&["step_a".into(), "step_b".into()], true)
        .await;

    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.success));
}

#[tokio::test]
async fn run_plan_raw_fail_fast_stops_on_first_failure() {
    let mut commands = HashMap::new();
    commands.insert("fail".to_string(), CommandSpec::Exec(false_cmd()));
    commands.insert("after".to_string(), CommandSpec::Exec(true_cmd()));
    let runner = test_runner(commands);

    let results = runner
        .run_plan_raw(&["fail".into(), "after".into()], true)
        .await;

    assert_eq!(results.len(), 1, "fail_fast must stop after first failure");
    assert!(!results[0].success);
}

#[tokio::test]
async fn run_plan_raw_unknown_command_returns_failure() {
    let runner = test_runner(HashMap::new());
    let results = runner.run_plan_raw(&["nope".into()], true).await;
    assert_eq!(results.len(), 1);
    assert!(!results[0].success);
    assert!(results[0].message.as_deref().unwrap().contains("unknown"));
}

#[tokio::test]
async fn run_raw_expands_composite_and_runs_leaves() {
    let mut commands = HashMap::new();
    commands.insert("a".to_string(), CommandSpec::Exec(true_cmd()));
    commands.insert("b".to_string(), CommandSpec::Exec(true_cmd()));
    commands.insert(
        "both".to_string(),
        CommandSpec::Composite(composite_cmd(&["a", "b"])),
    );
    let runner = test_runner(commands);

    let results = runner.run_raw("both").await.unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|r| r.success));
}
