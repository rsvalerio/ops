//! Tests for command execution.

use super::*;
use crate::command::events::RunnerEvent;
use crate::command::exec::{build_command, emit_output_events, exec_standalone};
use crate::command::results::StepResult;
use crate::test_support::{test_runner, EventAssertions};
use ops_core::config::CommandSpec;
use ops_core::test_utils::{
    composite_cmd, echo_cmd, exec_spec, exec_spec_with_cwd, false_cmd, parallel_cmd, sleep_cmd,
    true_cmd,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

fn runner_with_test_commands() -> CommandRunner {
    let mut commands = HashMap::new();
    commands.insert(
        "build".to_string(),
        CommandSpec::Exec(exec_spec("cargo", &["build"])),
    );
    commands.insert(
        "clippy".to_string(),
        CommandSpec::Exec(exec_spec("cargo", &["clippy"])),
    );
    commands.insert(
        "verify".to_string(),
        CommandSpec::Composite(composite_cmd(&["build", "clippy"])),
    );
    test_runner(commands)
}

#[test]
fn expand_to_leaves_single() {
    let runner = runner_with_test_commands();
    let plan = runner
        .expand_to_leaves("build")
        .expect("build must exist in test config");
    assert_eq!(plan, vec!["build"]);
}

#[test]
fn expand_to_leaves_composite() {
    let runner = runner_with_test_commands();
    let plan = runner
        .expand_to_leaves("verify")
        .expect("verify must exist in test config");
    assert_eq!(plan, vec!["build", "clippy"]);
}

#[test]
fn expand_to_leaves_unknown() {
    let runner = runner_with_test_commands();
    assert!(runner.expand_to_leaves("unknown").is_none());
}

#[test]
fn resolve_by_alias() {
    let mut commands = HashMap::new();
    let mut spec = exec_spec("cargo", &["build"]);
    spec.aliases = vec!["b".to_string(), "compile".to_string()];
    commands.insert("build".to_string(), CommandSpec::Exec(spec));
    let runner = test_runner(commands);

    // Resolve by canonical name
    assert!(runner.resolve("build").is_some());
    // Resolve by alias
    assert!(runner.resolve("b").is_some());
    assert!(runner.resolve("compile").is_some());
    // Unknown still returns None
    assert!(runner.resolve("unknown").is_none());
}

#[test]
fn expand_to_leaves_via_alias() {
    let mut commands = HashMap::new();
    let mut spec = exec_spec("cargo", &["build"]);
    spec.aliases = vec!["b".to_string()];
    commands.insert("build".to_string(), CommandSpec::Exec(spec));
    let runner = test_runner(commands);

    let plan = runner.expand_to_leaves("b").expect("alias must resolve");
    assert_eq!(plan, vec!["build"]);
}

#[tokio::test]
async fn run_plan_echo_success() {
    let mut runner = test_runner(HashMap::new());
    runner.register_commands(vec![("echo_hi".into(), CommandSpec::Exec(echo_cmd("hi")))]);
    let mut events = Vec::new();
    let results = runner
        .run_plan(&["echo_hi".into()], true, &mut |e| events.push(e))
        .await;
    assert!(results.iter().all(|r| r.success));
    events.assert_has_event(
        |e| matches!(e, RunnerEvent::StepFinished { .. }),
        "should have StepFinished event",
    );
    events.assert_has_event(
        |e| matches!(e, RunnerEvent::RunFinished { success: true, .. }),
        "should have RunFinished with success=true",
    );

    let result = &results[0];
    assert_eq!(result.id, "echo_hi");
    assert!(result.success);
    assert!(
        result.duration.as_millis() > 0,
        "should have non-zero duration"
    );
}

/// TQ-001: Uses 3s sleep with 1s timeout for reliable timing under CI load.
/// Previous 10s/1s was too slow; this provides 3x safety margin while staying fast.
///
/// # Timing Safety Margin
///
/// The 3s/1s ratio provides 3x safety margin for CI environments:
/// - Sleep duration: 3 seconds (command should run for 3s if not timed out)
/// - Timeout: 1 second (command is killed after 1s)
/// - Safety margin: 3x (timeout is 1/3 of sleep)
///
/// This is sufficient for most CI environments but may fail under extreme load.
/// Alternative approaches (mock time, deterministic test doubles) were considered
/// but would add significant complexity for a single test.
#[tokio::test]
async fn run_exec_timeout() {
    let runner = test_runner(HashMap::new());
    let mut spec = sleep_cmd(3);
    spec.timeout_secs = Some(1);
    let mut events = Vec::new();
    let result = runner
        .run_exec("sleep_cmd", &spec, &mut |e| events.push(e))
        .await;
    assert!(!result.success);
    assert!(result
        .message
        .as_ref()
        .is_some_and(|m| m.contains("timed out")));
    assert!(events
        .iter()
        .any(|e| matches!(e, RunnerEvent::StepFailed { .. })));
}

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
    assert!(
        events
            .iter()
            .any(|e| matches!(e, RunnerEvent::StepFinished { .. })),
        "should emit at least one StepFinished (ok command)"
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
    if let Some(RunnerEvent::StepFailed { message, .. }) = failed {
        assert!(message.contains("unknown command"), "message: {message}");
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

#[tokio::test]
async fn run_unknown_command_returns_error() {
    let runner = test_runner(HashMap::new());
    let mut events = Vec::new();
    let result = runner.run("nonexistent", &mut |e| events.push(e)).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread")]
async fn exec_standalone_skips_when_abort_set() {
    let (tx, mut rx) = mpsc::channel(16);
    let abort = Arc::new(AtomicBool::new(true));
    let spec = echo_cmd("should not run");
    let result = exec_standalone("skipped".into(), spec, PathBuf::from("."), tx, abort).await;
    assert!(result.success);
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

#[test]
fn step_result_failure_creates_correct_result() {
    let duration = Duration::from_millis(100);
    let result = StepResult::failure("test_cmd", duration, "test error".to_string());
    assert_eq!(result.id, "test_cmd");
    assert!(!result.success);
    assert_eq!(result.duration, duration);
    assert!(result.stdout.is_empty());
    assert!(result.stderr.is_empty());
    assert_eq!(result.message, Some("test error".to_string()));
}

#[test]
fn build_command_sets_program_and_args() {
    let spec = exec_spec("cargo", &["build", "--release"]);
    let cmd = build_command(&spec, std::path::Path::new("."));
    assert_eq!(cmd.as_std().get_program(), "cargo");
    let args: Vec<_> = cmd.as_std().get_args().collect();
    assert_eq!(args, vec!["build", "--release"]);
}

#[test]
fn build_command_uses_spec_cwd_when_provided() {
    let temp_dir = tempfile::tempdir().expect("tempdir");
    let mut spec = exec_spec("echo", &["test"]);
    spec.cwd = Some(temp_dir.path().to_path_buf());
    let cmd = build_command(&spec, std::path::Path::new("."));
    assert_eq!(cmd.as_std().get_current_dir(), Some(temp_dir.path()));
}

#[test]
fn emit_output_events_emits_stdout_and_stderr() {
    let mut events: Vec<RunnerEvent> = Vec::new();
    emit_output_events("test", "line1\nline2\n", "err1\n", &mut |e| events.push(e));

    let stdout_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, RunnerEvent::StepOutput { stderr: false, .. }))
        .collect();
    let stderr_events: Vec<_> = events
        .iter()
        .filter(|e| matches!(e, RunnerEvent::StepOutput { stderr: true, .. }))
        .collect();

    assert_eq!(stdout_events.len(), 2);
    assert_eq!(stderr_events.len(), 1);
}

mod proptest_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn expand_to_leaves_single_exec_returns_self(id in "[a-zA-Z_][a-zA-Z0-9_]{0,10}") {
            let mut commands = HashMap::new();
            commands.insert(
                id.clone(),
                CommandSpec::Exec(exec_spec("cargo", &["build"])),
            );
            let runner = test_runner(commands);
            let result = runner.expand_to_leaves(&id);
            prop_assert!(result.is_some());
            prop_assert_eq!(result.unwrap(), vec![id]);
        }

        #[test]
        fn expand_to_leaves_composite_flattens(
            name in "grp[a-zA-Z0-9_]{0,5}",
            cmd1 in "a[a-zA-Z0-9_]{0,5}",
            cmd2 in "b[a-zA-Z0-9_]{0,5}"
        ) {
            let mut commands = HashMap::new();
            commands.insert(cmd1.clone(), CommandSpec::Exec(exec_spec("echo", &[&cmd1])));
            commands.insert(cmd2.clone(), CommandSpec::Exec(exec_spec("echo", &[&cmd2])));
            commands.insert(
                name.clone(),
                CommandSpec::Composite(ops_core::config::CompositeCommandSpec {
                    commands: vec![cmd1.clone(), cmd2.clone()],
                    parallel: false,
                    fail_fast: true,
                    help: None,
                    aliases: Vec::new(),
                    category: None,
                }),
            );
            let runner = test_runner(commands);
            let result = runner.expand_to_leaves(&name);
            prop_assert!(result.is_some());
            let leaves = result.unwrap();
            prop_assert!(leaves.iter().any(|l| l == cmd1.as_str()));
            prop_assert!(leaves.iter().any(|l| l == cmd2.as_str()));
            prop_assert!(!leaves.iter().any(|l| l == name.as_str()));
        }

        #[test]
        fn expand_to_leaves_unknown_returns_none(id in "unknown[a-zA-Z0-9_]{0,8}") {
            let runner = test_runner(HashMap::new());
            let result = runner.expand_to_leaves(&id);
            prop_assert!(result.is_none());
        }
    }
}

mod exec_unit_tests {
    use super::*;
    use crate::command::exec::{
        build_step_result, emit_output_events, emit_step_completion, execute_with_timeout,
    };
    use crate::command::results::CommandOutput;

    #[tokio::test]
    async fn execute_with_timeout_no_timeout_succeeds() {
        let mut cmd = tokio::process::Command::new("echo");
        cmd.arg("test");
        let result = execute_with_timeout(cmd, None).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.status.success());
    }

    #[tokio::test]
    async fn execute_with_timeout_with_timeout_returns_output() {
        let mut cmd = tokio::process::Command::new("echo");
        cmd.arg("test");
        let result = execute_with_timeout(cmd, Some(Duration::from_secs(5))).await;
        assert!(result.is_ok());
    }

    #[test]
    fn emit_step_completion_success() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        let output = CommandOutput {
            success: true,
            stdout: "out".to_string(),
            stderr: String::new(),
            status_message: "exit status: 0".to_string(),
        };
        emit_step_completion(
            "test",
            Duration::from_millis(100),
            &output,
            Some("echo test".to_string()),
            &mut |e| events.push(e),
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            RunnerEvent::StepFinished { id, duration_secs, .. }
            if id == "test" && *duration_secs > 0.0
        ));
    }

    #[test]
    fn emit_step_completion_failure() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        let output = CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: "error".to_string(),
            status_message: "exit status: 1".to_string(),
        };
        emit_step_completion(
            "fail_cmd",
            Duration::from_millis(50),
            &output,
            Some("false".to_string()),
            &mut |e| events.push(e),
        );
        assert_eq!(events.len(), 1);
        assert!(matches!(
            &events[0],
            RunnerEvent::StepFailed { id, message, .. }
            if id == "fail_cmd" && message.contains("exit status")
        ));
    }

    #[test]
    fn build_step_result_from_success_output() {
        let output = CommandOutput {
            success: true,
            stdout: "stdout content".to_string(),
            stderr: String::new(),
            status_message: "exit status: 0".to_string(),
        };
        let result = build_step_result("cmd", Duration::from_millis(200), output);
        assert_eq!(result.id, "cmd");
        assert!(result.success);
        assert_eq!(result.duration, Duration::from_millis(200));
        assert_eq!(result.stdout, "stdout content");
        assert!(result.message.is_none());
    }

    #[test]
    fn build_step_result_from_failure_output() {
        let output = CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: "stderr content".to_string(),
            status_message: "exit status: 1".to_string(),
        };
        let result = build_step_result("fail", Duration::from_millis(100), output);
        assert_eq!(result.id, "fail");
        assert!(!result.success);
        assert_eq!(result.stderr, "stderr content");
        assert_eq!(result.message, Some("exit status: 1".to_string()));
    }

    #[test]
    fn emit_output_events_empty_input() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        emit_output_events("test", "", "", &mut |e| events.push(e));
        assert!(events.is_empty(), "empty input should produce no events");
    }

    #[test]
    fn emit_output_events_crlf_handling() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        emit_output_events("test", "line1\r\nline2\r\n", "err\r\n", &mut |e| {
            events.push(e)
        });

        let stdout_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, RunnerEvent::StepOutput { stderr: false, .. }))
            .collect();
        let stderr_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, RunnerEvent::StepOutput { stderr: true, .. }))
            .collect();

        assert_eq!(stdout_events.len(), 2);
        assert_eq!(stderr_events.len(), 1);
    }

    #[test]
    fn emit_output_events_trailing_newline() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        emit_output_events("test", "line1\nline2\n\n", "", &mut |e| events.push(e));

        let stdout_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, RunnerEvent::StepOutput { stderr: false, .. }))
            .collect();

        assert_eq!(
            stdout_events.len(),
            3,
            "should include empty line from trailing newline"
        );
    }

    #[test]
    fn build_command_includes_env_vars() {
        let mut spec = exec_spec("echo", &["test"]);
        spec.env
            .insert("MY_VAR".to_string(), "my_value".to_string());
        let cmd = build_command(&spec, std::path::Path::new("."));
        let env = cmd.as_std().get_envs().collect::<Vec<_>>();
        assert!(
            env.iter().any(|(k, _)| k.to_string_lossy() == "MY_VAR"),
            "env var should be set"
        );
    }
}

/// TQ-005: Tests for build_command error paths.
mod build_command_error_tests {
    use super::*;
    use crate::command::exec::build_command;

    #[test]
    fn build_command_with_nonexistent_cwd_still_builds() {
        let mut spec = exec_spec("echo", &["test"]);
        spec.cwd = Some(PathBuf::from("/nonexistent/path/that/does/not/exist"));
        let cmd = build_command(&spec, std::path::Path::new("."));
        assert_eq!(cmd.as_std().get_program(), "echo");
    }

    #[test]
    fn build_command_with_relative_cwd() {
        let mut spec = exec_spec("echo", &["test"]);
        spec.cwd = Some(PathBuf::from("relative/path"));
        let cmd = build_command(&spec, std::path::Path::new("/base"));
        let current_dir = cmd.as_std().get_current_dir();
        assert_eq!(
            current_dir,
            Some(std::path::Path::new("/base/relative/path"))
        );
    }

    #[test]
    fn build_command_with_absolute_cwd() {
        let mut spec = exec_spec("echo", &["test"]);
        spec.cwd = Some(PathBuf::from("/absolute/path"));
        let cmd = build_command(&spec, std::path::Path::new("/base"));
        let current_dir = cmd.as_std().get_current_dir();
        assert_eq!(current_dir, Some(std::path::Path::new("/absolute/path")));
    }

    #[test]
    fn build_command_with_empty_args() {
        let spec = exec_spec("echo", &[]);
        let cmd = build_command(&spec, std::path::Path::new("."));
        assert_eq!(cmd.as_std().get_program(), "echo");
        let args: Vec<_> = cmd.as_std().get_args().collect();
        assert!(args.is_empty());
    }

    #[test]
    fn build_command_with_many_args() {
        let spec = exec_spec("echo", &["a", "b", "c", "d", "e"]);
        let cmd = build_command(&spec, std::path::Path::new("."));
        let args: Vec<_> = cmd.as_std().get_args().collect();
        assert_eq!(args.len(), 5);
    }

    #[test]
    fn build_command_with_special_chars_in_args() {
        let spec = exec_spec(
            "echo",
            &["arg with spaces", "arg'with'quotes", "arg\"with\"double"],
        );
        let cmd = build_command(&spec, std::path::Path::new("."));
        let args: Vec<_> = cmd.as_std().get_args().collect();
        assert_eq!(args.len(), 3);
        assert_eq!(args[0], "arg with spaces");
        assert_eq!(args[1], "arg'with'quotes");
        assert_eq!(args[2], "arg\"with\"double");
    }
}

/// TQ-018: Tests for emit_output_events edge cases.
mod emit_output_edge_tests {
    use super::*;
    use crate::command::exec::emit_output_events;

    #[test]
    fn emit_output_events_with_very_long_line() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        let long_line = "x".repeat(100_000);
        emit_output_events("test", &long_line, "", &mut |e| events.push(e));

        assert_eq!(events.len(), 1);
        if let RunnerEvent::StepOutput { line, .. } = &events[0] {
            assert_eq!(line.len(), 100_000);
        } else {
            panic!("expected StepOutput event");
        }
    }

    #[test]
    fn emit_output_events_with_many_lines() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        let many_lines: String = (0..1000).map(|i| format!("line{}\n", i)).collect();
        emit_output_events("test", &many_lines, "", &mut |e| events.push(e));

        assert_eq!(events.len(), 1000);
    }

    #[test]
    fn emit_output_events_with_unicode() {
        let mut events: Vec<RunnerEvent> = Vec::new();
        let unicode = "\u{65E5}\u{672C}\u{8A9E}\n\u{30C6}\u{30B9}\u{30C8}\n\u{1F389}\n";
        emit_output_events("test", unicode, "", &mut |e| events.push(e));

        assert_eq!(events.len(), 3);
    }
}

mod nested_composite_tests {
    use super::*;

    #[test]
    fn expand_to_leaves_deeply_nested_composite() {
        let mut commands = HashMap::new();
        commands.insert("leaf1".to_string(), CommandSpec::Exec(echo_cmd("1")));
        commands.insert("leaf2".to_string(), CommandSpec::Exec(echo_cmd("2")));
        commands.insert("leaf3".to_string(), CommandSpec::Exec(echo_cmd("3")));

        commands.insert(
            "level2_a".to_string(),
            CommandSpec::Composite(composite_cmd(&["leaf1", "leaf2"])),
        );
        commands.insert(
            "level2_b".to_string(),
            CommandSpec::Composite(composite_cmd(&["leaf3"])),
        );
        commands.insert(
            "level3".to_string(),
            CommandSpec::Composite(composite_cmd(&["level2_a", "level2_b"])),
        );

        let runner = test_runner(commands);
        let plan = runner.expand_to_leaves("level3").expect("should resolve");
        assert_eq!(plan, vec!["leaf1", "leaf2", "leaf3"]);
    }

    #[test]
    fn expand_to_leaves_nested_missing_intermediate() {
        let mut commands = HashMap::new();
        commands.insert("leaf".to_string(), CommandSpec::Exec(echo_cmd("1")));
        commands.insert(
            "level2".to_string(),
            CommandSpec::Composite(composite_cmd(&["nonexistent"])),
        );
        commands.insert(
            "level3".to_string(),
            CommandSpec::Composite(composite_cmd(&["level2"])),
        );

        let runner = test_runner(commands);
        assert!(
            runner.expand_to_leaves("level3").is_none(),
            "missing intermediate command should return None"
        );
    }

    #[test]
    fn expand_to_leaves_deep_cycle() {
        let mut commands = HashMap::new();
        commands.insert("leaf".to_string(), CommandSpec::Exec(echo_cmd("1")));
        commands.insert(
            "level2".to_string(),
            CommandSpec::Composite(composite_cmd(&["level3"])),
        );
        commands.insert(
            "level3".to_string(),
            CommandSpec::Composite(composite_cmd(&["level2"])),
        );

        let runner = test_runner(commands);
        assert!(
            runner.expand_to_leaves("level2").is_none(),
            "deep cycle should return None"
        );
    }
}

mod error_path_tests {
    use super::*;

    #[tokio::test]
    async fn run_exec_nonexistent_program() {
        let runner = test_runner(HashMap::new());
        let spec = exec_spec("nonexistent_program_xyz123", &[]);
        let mut events = Vec::new();
        let result = runner
            .run_exec("bad_cmd", &spec, &mut |e| events.push(e))
            .await;
        assert!(!result.success, "should fail for nonexistent program");
        assert!(result.message.is_some());
        assert!(
            events
                .iter()
                .any(|e| matches!(e, RunnerEvent::StepFailed { .. })),
            "should emit StepFailed event"
        );
    }

    #[tokio::test]
    async fn run_exec_invalid_cwd() {
        let runner = test_runner(HashMap::new());
        let spec = exec_spec_with_cwd(
            "echo",
            &["test"],
            Some(PathBuf::from("/nonexistent/directory/xyz123")),
        );
        let mut events = Vec::new();
        let result = runner
            .run_exec("bad_cwd", &spec, &mut |e| events.push(e))
            .await;
        assert!(!result.success, "should fail for invalid cwd");
        assert!(result.message.is_some());
    }

    /// TQ-001: Permission denied error path (non-executable file).
    #[cfg(unix)]
    #[tokio::test]
    async fn run_exec_permission_denied() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("not_executable.sh");
        {
            let mut f = std::fs::File::create(&script).unwrap();
            f.write_all(b"#!/bin/sh\necho hello\n").unwrap();
            // Deliberately NOT setting execute permission
        }
        let runner = test_runner(HashMap::new());
        let spec = exec_spec(script.to_str().unwrap(), &[]);
        let mut events = Vec::new();
        let result = runner
            .run_exec("perm_denied", &spec, &mut |e| events.push(e))
            .await;
        assert!(!result.success, "should fail for non-executable file");
        assert!(result.message.is_some());
        assert!(
            events
                .iter()
                .any(|e| matches!(e, RunnerEvent::StepFailed { .. })),
            "should emit StepFailed event for permission denied"
        );
    }

    #[test]
    fn list_command_ids_includes_all_commands() {
        let mut commands = HashMap::new();
        commands.insert(
            "build".to_string(),
            CommandSpec::Exec(exec_spec("cargo", &["build"])),
        );
        commands.insert(
            "test".to_string(),
            CommandSpec::Exec(exec_spec("cargo", &["test"])),
        );
        let runner = test_runner(commands);
        let ids = runner.list_command_ids();
        assert!(ids.contains(&"build".into()));
        assert!(ids.contains(&"test".into()));
        assert!(
            ids.len() >= 2,
            "should have at least build and test commands"
        );
    }

    #[test]
    fn list_command_ids_with_extension_commands() {
        let mut runner = test_runner(HashMap::new());
        runner.register_commands(vec![(
            "ext_cmd".into(),
            CommandSpec::Exec(exec_spec("echo", &["ext"])),
        )]);
        let ids = runner.list_command_ids();
        assert!(ids.contains(&"ext_cmd".into()));
    }
}

mod cycle_detection_tests {
    use super::*;

    #[test]
    fn expand_to_leaves_cycle_2_nodes() {
        let mut commands = HashMap::new();
        commands.insert(
            "a".to_string(),
            CommandSpec::Composite(composite_cmd(&["b"])),
        );
        commands.insert(
            "b".to_string(),
            CommandSpec::Composite(composite_cmd(&["a"])),
        );
        let runner = test_runner(commands);
        assert!(
            runner.expand_to_leaves("a").is_none(),
            "2-node cycle should return None"
        );
    }

    #[test]
    fn expand_to_leaves_cycle_3_nodes() {
        let mut commands = HashMap::new();
        commands.insert(
            "a".to_string(),
            CommandSpec::Composite(composite_cmd(&["b"])),
        );
        commands.insert(
            "b".to_string(),
            CommandSpec::Composite(composite_cmd(&["c"])),
        );
        commands.insert(
            "c".to_string(),
            CommandSpec::Composite(composite_cmd(&["a"])),
        );
        let runner = test_runner(commands);
        assert!(
            runner.expand_to_leaves("a").is_none(),
            "3-node cycle a->b->c->a should return None"
        );
    }

    #[test]
    fn expand_to_leaves_self_reference() {
        let mut commands = HashMap::new();
        commands.insert(
            "self_ref".to_string(),
            CommandSpec::Composite(composite_cmd(&["self_ref"])),
        );
        let runner = test_runner(commands);
        assert!(
            runner.expand_to_leaves("self_ref").is_none(),
            "self-referencing command should return None"
        );
    }
}

/// TQ-013: Verify parallel execution actually runs commands concurrently.
mod parallel_timing_tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn run_plan_parallel_executes_concurrently() {
        let mut commands = HashMap::new();
        commands.insert("sleep_a".to_string(), CommandSpec::Exec(sleep_cmd(1)));
        commands.insert("sleep_b".to_string(), CommandSpec::Exec(sleep_cmd(1)));
        let runner = test_runner(commands);
        let mut events = Vec::new();
        let start = std::time::Instant::now();
        let results = runner
            .run_plan_parallel(&["sleep_a".into(), "sleep_b".into()], true, &mut |e| {
                events.push(e)
            })
            .await;
        let elapsed = start.elapsed();

        assert!(
            results.iter().all(|r| r.success),
            "both sleeps should succeed"
        );
        // Two 1-second sleeps running in parallel should finish in ~1s, not ~2s.
        // Use 1.8s as threshold to give ample margin for CI overhead.
        assert!(
            elapsed.as_secs_f64() < 1.8,
            "parallel execution took {:.2}s -- expected < 1.8s (two 1s sleeps in parallel)",
            elapsed.as_secs_f64()
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

mod parallel_infra_tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::Arc;
    use tokio::sync::mpsc;

    #[tokio::test(flavor = "multi_thread")]
    async fn spawn_parallel_tasks_creates_correct_count() {
        let steps: Vec<(CommandId, _)> = vec![
            ("cmd1".into(), echo_cmd("a")),
            ("cmd2".into(), echo_cmd("b")),
            ("cmd3".into(), echo_cmd("c")),
        ];
        let (rx, _abort, join_set) = CommandRunner::spawn_parallel_tasks(steps, PathBuf::from("."));
        drop(rx);
        let results = CommandRunner::collect_join_results(join_set).await;
        assert_eq!(results.len(), 3);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn handle_parallel_events_receives_all() {
        let (tx, rx) = mpsc::channel(16);
        let abort = Arc::new(AtomicBool::new(false));

        tx.send(RunnerEvent::StepStarted {
            id: "a".into(),
            display_cmd: None,
        })
        .await
        .unwrap();
        tx.send(RunnerEvent::StepFinished {
            id: "a".into(),
            duration_secs: 0.1,
            display_cmd: None,
        })
        .await
        .unwrap();
        drop(tx);

        let mut events = Vec::new();
        CommandRunner::handle_parallel_events(rx, false, abort, &mut |e| events.push(e)).await;

        assert_eq!(events.len(), 2);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn handle_parallel_events_sets_abort_on_fail_fast() {
        let (tx, rx) = mpsc::channel(16);
        let abort = Arc::new(AtomicBool::new(false));

        tx.send(RunnerEvent::StepFailed {
            id: "fail".into(),
            duration_secs: 0.1,
            message: "error".into(),
            display_cmd: None,
        })
        .await
        .unwrap();
        drop(tx);

        let mut events = Vec::new();
        CommandRunner::handle_parallel_events(rx, true, Arc::clone(&abort), &mut |e| {
            events.push(e)
        })
        .await;

        assert!(
            abort.load(std::sync::atomic::Ordering::Acquire),
            "abort should be set on failure with fail_fast=true"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn handle_parallel_events_no_abort_without_fail_fast() {
        let (tx, rx) = mpsc::channel(16);
        let abort = Arc::new(AtomicBool::new(false));

        tx.send(RunnerEvent::StepFailed {
            id: "fail".into(),
            duration_secs: 0.1,
            message: "error".into(),
            display_cmd: None,
        })
        .await
        .unwrap();
        drop(tx);

        let mut events = Vec::new();
        CommandRunner::handle_parallel_events(rx, false, Arc::clone(&abort), &mut |e| {
            events.push(e)
        })
        .await;

        assert!(
            !abort.load(std::sync::atomic::Ordering::Acquire),
            "abort should NOT be set without fail_fast"
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn collect_join_results_handles_panics() {
        let mut join_set = tokio::task::JoinSet::new();
        join_set.spawn(async { panic!("test panic message") });
        join_set.spawn(async { StepResult::success("ok", Duration::from_millis(10)) });

        let results = CommandRunner::collect_join_results(join_set).await;

        assert_eq!(results.len(), 2);
        let panic_result = results.iter().find(|r| r.id == "<panicked>");
        assert!(panic_result.is_some(), "should have panic result");
        assert!(!panic_result.unwrap().success);
        assert!(
            panic_result
                .unwrap()
                .message
                .as_ref()
                .unwrap()
                .contains("test panic message"),
            "panic message should be propagated to StepResult.message"
        );
    }
}

/// TQ-012: Tests for depth limit in expand_to_leaves.
mod depth_limit_tests {
    use super::*;

    fn create_nested_commands(depth: usize) -> HashMap<String, CommandSpec> {
        let mut commands = HashMap::new();
        for i in 0..depth {
            let name = format!("level_{}", i);
            let next_name = format!("level_{}", i + 1);
            commands.insert(
                name,
                CommandSpec::Composite(ops_core::config::CompositeCommandSpec {
                    commands: vec![next_name],
                    parallel: false,
                    fail_fast: true,
                    help: None,
                    aliases: Vec::new(),
                    category: None,
                }),
            );
        }
        commands.insert(
            format!("level_{}", depth),
            CommandSpec::Exec(exec_spec("echo", &["leaf"])),
        );
        commands
    }

    #[test]
    fn expand_to_leaves_shallow_nesting_succeeds() {
        let commands = create_nested_commands(10);
        let runner = test_runner(commands);
        let result = runner.expand_to_leaves("level_0");
        assert!(result.is_some(), "10 levels should be well within limit");
    }

    #[test]
    fn expand_to_leaves_at_depth_limit_succeeds() {
        let commands = create_nested_commands(99);
        let runner = test_runner(commands);
        let result = runner.expand_to_leaves("level_0");
        assert!(
            result.is_some(),
            "99 levels (depth=99 starting from 0) should succeed at MAX_DEPTH=100"
        );
    }

    #[test]
    fn expand_to_leaves_exceeds_depth_limit_returns_none() {
        let commands = create_nested_commands(101);
        let runner = test_runner(commands);
        let result = runner.expand_to_leaves("level_0");
        assert!(
            result.is_none(),
            "101 levels (exceeds MAX_DEPTH=100) should return None"
        );
    }
}

mod sensitive_env_tests {
    use crate::command::exec::{
        has_high_entropy, is_sensitive_env_key, looks_like_aws_key, looks_like_jwt,
        looks_like_secret_value, looks_like_uuid,
    };

    #[test]
    fn has_high_entropy_detects_random_strings() {
        assert!(has_high_entropy("AbCdEfGh123456789XyZ"));
        assert!(has_high_entropy("aB1cD2eF3gH4iJ5kL6"));
    }

    #[test]
    fn has_high_entropy_rejects_simple_strings() {
        assert!(!has_high_entropy("simple"));
        assert!(!has_high_entropy("aaaaaaaaaaaa"));
        assert!(!has_high_entropy("12345678901234567890"));
    }

    #[test]
    fn looks_like_jwt_detects_jwt_format() {
        assert!(looks_like_jwt(
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.signature"
        ));
        assert!(looks_like_jwt(
            "eyJzdWIiOiIxMjM0In0.eyJuYW1lIjoiSm9obiJ9.sig"
        ));
    }

    #[test]
    fn looks_like_jwt_rejects_non_jwt() {
        assert!(!looks_like_jwt("not-a-jwt"));
        assert!(!looks_like_jwt("bearer eyJhbGciOiJIUzI1NiJ9"));
    }

    #[test]
    fn looks_like_aws_key_detects_40_char_keys() {
        assert!(looks_like_aws_key(
            "AKIAIOSFODNN7EXAMPLE12345678901234567890"
        ));
        assert!(looks_like_aws_key(
            "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY"
        ));
    }

    #[test]
    fn looks_like_aws_key_rejects_wrong_length() {
        assert!(!looks_like_aws_key("AKIAIOSFODNN7EXAMPLE"));
        assert!(!looks_like_aws_key("short"));
    }

    #[test]
    fn looks_like_uuid_detects_uuid_format() {
        assert!(looks_like_uuid("550e8400-e29b-41d4-a716-446655440000"));
        assert!(looks_like_uuid("123e4567-e89b-12d3-a456-426614174000"));
    }

    #[test]
    fn looks_like_uuid_rejects_non_uuid() {
        assert!(!looks_like_uuid("not-a-uuid"));
        assert!(!looks_like_uuid("550e8400-e29b-41d4-a716"));
    }

    #[test]
    fn looks_like_uuid_rejects_wrong_segment_lengths() {
        // Right length and hyphen count, but wrong segment layout (not 8-4-4-4-12)
        assert!(!looks_like_uuid("550e84001-e29b-41d4-a71-446655440000"));
        // Contains non-hex characters
        assert!(!looks_like_uuid("550e8400-e29b-41d4-a716-44665544000g"));
    }

    #[test]
    fn looks_like_secret_value_combines_all_checks() {
        assert!(looks_like_secret_value(
            "eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0In0.signature"
        ));
        assert!(looks_like_secret_value(
            "550e8400-e29b-41d4-a716-446655440000"
        ));
        assert!(looks_like_secret_value(
            "AKIAIOSFODNN7EXAMPLE12345678901234567890"
        ));
        assert!(looks_like_secret_value("AbCdEfGh123456789XyZ12345"));
    }

    #[test]
    fn looks_like_secret_value_rejects_short_values() {
        assert!(!looks_like_secret_value("short"));
        assert!(!looks_like_secret_value("1234567890"));
    }

    #[test]
    fn is_sensitive_env_key_case_insensitive() {
        assert!(is_sensitive_env_key("PASSWORD"));
        assert!(is_sensitive_env_key("password"));
        assert!(is_sensitive_env_key("PaSsWoRd"));
        assert!(is_sensitive_env_key("MY_API_KEY"));
        assert!(is_sensitive_env_key("my_api_key"));
    }
}

/// TQ-GAP-005: Tests for CommandRunner::query_data().
mod query_data_tests {
    use super::*;
    use ops_extension::{Context, DataProvider, DataProviderError, DataRegistry};

    struct FixedProvider {
        value: serde_json::Value,
    }

    impl DataProvider for FixedProvider {
        fn name(&self) -> &'static str {
            "fixed"
        }
        fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            Ok(self.value.clone())
        }
    }

    struct FailingProvider;

    impl DataProvider for FailingProvider {
        fn name(&self) -> &'static str {
            "failing"
        }
        fn provide(&self, _ctx: &mut Context) -> Result<serde_json::Value, DataProviderError> {
            Err(DataProviderError::computation_failed("provider error"))
        }
    }

    #[test]
    fn query_data_returns_provider_value() {
        let mut registry = DataRegistry::new();
        registry.register(
            "fixed",
            Box::new(FixedProvider {
                value: serde_json::json!({"hello": "world"}),
            }),
        );
        let mut runner = test_runner(HashMap::new());
        runner.register_data_providers(registry);

        let result = runner.query_data("fixed");
        assert!(result.is_ok());
        assert_eq!(*result.unwrap(), serde_json::json!({"hello": "world"}));
    }

    #[test]
    fn query_data_caches_results() {
        let mut registry = DataRegistry::new();
        registry.register(
            "fixed",
            Box::new(FixedProvider {
                value: serde_json::json!(42),
            }),
        );
        let mut runner = test_runner(HashMap::new());
        runner.register_data_providers(registry);

        let v1 = runner.query_data("fixed").expect("first call");
        let v2 = runner.query_data("fixed").expect("second call (cached)");
        assert_eq!(*v1, *v2);
    }

    #[test]
    fn query_data_unknown_provider_errors() {
        let mut runner = test_runner(HashMap::new());
        let result = runner.query_data("nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn query_data_failing_provider_errors() {
        let mut registry = DataRegistry::new();
        registry.register("failing", Box::new(FailingProvider));
        let mut runner = test_runner(HashMap::new());
        runner.register_data_providers(registry);

        let result = runner.query_data("failing");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("provider error"));
    }
}
