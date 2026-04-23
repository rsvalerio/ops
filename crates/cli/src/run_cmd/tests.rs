use super::*;
use crate::run_cmd::dry_run::run_command_dry_run_to;
use crate::run_cmd::plan::{build_display_map, display_cmd_for, log_step_results};
use crate::test_utils::{exec_spec, TestConfigBuilder};
use std::path::PathBuf;

// -- display_cmd_for --

#[test]
fn display_cmd_for_exec_command() {
    let config = TestConfigBuilder::new()
        .exec("build", "cargo", &["build", "--all"])
        .build();
    let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
    assert_eq!(display_cmd_for(&runner, "build"), "cargo build --all");
}

#[test]
fn display_cmd_for_unknown_returns_id() {
    let config = ops_core::config::Config::default();
    let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
    assert_eq!(display_cmd_for(&runner, "missing"), "missing");
}

#[test]
fn display_cmd_for_composite_returns_id() {
    let config = TestConfigBuilder::new()
        .composite("verify", &["build", "test"])
        .build();
    let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
    assert_eq!(display_cmd_for(&runner, "verify"), "verify");
}

/// TQ-007: Full lifecycle integration test.
///
/// This test validates the complete command execution path:
/// - Config loading
/// - Extension setup
/// - Command resolution and execution
/// - Event emission
/// - Result aggregation
///
/// It is ignored because it:
/// - Spawns real subprocesses
/// - Writes to stderr (visible in test output)
/// - Requires `echo` to be available
///
/// **Re-enable criteria:**
/// - Run with `cargo test -- --ignored` in environments with echo available
/// - Or mock subprocess execution using a trait-based approach
///
/// **Tracking:** Run periodically in CI to validate full integration.
#[tokio::test(flavor = "multi_thread")]
#[ignore = "TQ-007: spawns real subprocesses; run with --ignored. Validates full CLI lifecycle."]
async fn run_command_cli_full_lifecycle() {
    let (_dir, _guard) = crate::test_utils::with_temp_config(
        r#"
[output]
theme = "compact"
columns = 80

[commands.echo_test]
program = "echo"
args = ["integration_test"]
"#,
    );

    let cwd = std::env::current_dir().expect("cwd");
    let config = ops_core::config::load_config().expect("load_config");
    let mut runner = ops_runner::command::CommandRunner::new(config, cwd);
    setup_extensions(&mut runner).expect("setup_extensions");

    let mut events = Vec::new();
    let results = runner
        .run("echo_test", &mut |e| events.push(e))
        .await
        .expect("run should succeed");

    assert!(
        results.iter().all(|r| r.success),
        "all steps should succeed"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ops_runner::command::RunnerEvent::PlanStarted { .. })),
        "should emit PlanStarted"
    );
    assert!(
        events
            .iter()
            .any(|e| matches!(e, ops_runner::command::RunnerEvent::StepFinished { .. })),
        "should emit StepFinished"
    );
    assert!(
        events.iter().any(|e| matches!(
            e,
            ops_runner::command::RunnerEvent::RunFinished { success: true, .. }
        )),
        "should emit RunFinished with success"
    );
}

mod run_command_tests {
    use super::*;

    #[test]
    fn run_command_returns_error_for_unknown_command() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.echo_test]
program = "echo"
args = ["test"]
"#,
        );

        let result = run_command("nonexistent", false, false, None, false);
        assert!(
            result.is_err(),
            "run_command should return error for unknown command"
        );
    }

    #[test]
    fn run_command_returns_success_for_valid_command() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.echo_test]
program = "echo"
args = ["test"]
"#,
        );

        let result = run_command("echo_test", false, false, None, false);
        assert!(result.is_ok(), "run_command should not error");
        let exit_code = result.unwrap();
        assert_eq!(
            exit_code,
            ExitCode::SUCCESS,
            "valid command should return SUCCESS"
        );
    }

    #[test]
    fn run_command_returns_failure_for_failing_command() {
        let fail_cmd = if cfg!(windows) {
            r#"program = "cmd"
args = ["/C", "exit", "1"]"#
        } else {
            r#"program = "false"
args = []"#
        };
        let (_dir, _guard) =
            crate::test_utils::with_temp_config(&format!("[commands.fail_cmd]\n{}\n", fail_cmd));

        let result = run_command("fail_cmd", false, false, None, false);
        assert!(result.is_ok(), "run_command should not error");
        let exit_code = result.unwrap();
        assert_eq!(
            exit_code,
            ExitCode::FAILURE,
            "failing command should return FAILURE"
        );
    }

    #[test]
    fn run_command_returns_error_for_cycle() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.a]
commands = ["b"]

[commands.b]
commands = ["a"]
"#,
        );

        let result = run_command("a", false, false, None, false);
        assert!(result.is_err(), "run_command should return error for cycle");
    }
}

mod build_display_map_tests {
    use super::*;

    #[test]
    fn build_display_map_with_config_commands() {
        let config = crate::test_utils::TestConfigBuilder::new()
            .exec("build", "cargo", &["build"])
            .exec("test", "cargo", &["test"])
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let display_map = build_display_map(&runner, &["build".into(), "test".into()]);

        assert_eq!(display_map.get("build"), Some(&"cargo build".to_string()));
        assert_eq!(display_map.get("test"), Some(&"cargo test".to_string()));
    }

    #[test]
    fn build_display_map_with_unknown_command() {
        let config = ops_core::config::Config::default();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let display_map = build_display_map(&runner, &["unknown".into()]);

        assert_eq!(display_map.get("unknown"), Some(&"unknown".to_string()));
    }

    #[test]
    fn build_display_map_with_composite_command() {
        let config = crate::test_utils::TestConfigBuilder::new()
            .composite("verify", &["build", "test"])
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let display_map = build_display_map(&runner, &["verify".into()]);

        assert_eq!(display_map.get("verify"), Some(&"verify".to_string()));
    }

    #[test]
    fn display_cmd_for_with_extension_command() {
        let mut runner = ops_runner::command::CommandRunner::new(
            ops_core::config::Config::default(),
            PathBuf::from("."),
        );
        runner.register_commands(vec![(
            "ext_cmd".into(),
            ops_core::config::CommandSpec::Exec(ops_core::config::ExecCommandSpec::new(
                "echo",
                ["ext"],
            )),
        )]);

        assert_eq!(display_cmd_for(&runner, "ext_cmd"), "echo ext");
    }
}

// -- run_external_command --

mod run_external_command_tests {
    use super::*;

    #[test]
    fn run_external_command_empty_args_errors() {
        let args: Vec<OsString> = vec![];
        let result = run_external_command(&args, false, false, None, false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing command"));
    }

    #[test]
    fn run_external_command_single_command_dry_run() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.echo_test]
program = "echo"
args = ["hello"]
"#,
        );
        let args: Vec<OsString> = vec![OsString::from("echo_test")];
        let result = run_external_command(&args, true, false, None, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ExitCode::SUCCESS);
    }

    #[test]
    fn run_external_command_multi_command_dry_run() {
        let (_dir, _guard) = crate::test_utils::with_temp_config(
            r#"
[commands.build]
program = "echo"
args = ["build"]

[commands.test]
program = "echo"
args = ["test"]
"#,
        );
        let args: Vec<OsString> = vec![OsString::from("build"), OsString::from("test")];
        let result = run_external_command(&args, true, false, None, false);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ExitCode::SUCCESS);
    }

    #[test]
    fn run_external_command_single_unknown_errors() {
        let (_dir, _guard) = crate::test_utils::with_temp_config("");
        let args: Vec<OsString> = vec![OsString::from("nonexistent")];
        let result = run_external_command(&args, false, false, None, false);
        assert!(result.is_err());
    }
}

// -- log_step_results --

#[test]
fn log_step_results_does_not_panic() {
    let results = vec![StepResult {
        id: "test".into(),
        success: true,
        duration: std::time::Duration::from_millis(100),
        stdout: "output".to_string(),
        stderr: String::new(),
        message: None,
    }];
    log_step_results(&results);
}

#[test]
fn log_step_results_empty() {
    log_step_results(&[]);
}

mod run_command_dry_run_tests {
    use super::*;

    fn build_test_runner() -> ops_runner::command::CommandRunner {
        let config = TestConfigBuilder::new()
            .exec("build", "cargo", &["build", "--all"])
            .exec("test", "cargo", &["test"])
            .command(
                "verify",
                ops_core::config::CommandSpec::Composite(
                    ops_core::config::CompositeCommandSpec::new(["build", "test"]),
                ),
            )
            .build();
        ops_runner::command::CommandRunner::new(config, PathBuf::from("."))
    }

    #[test]
    fn dry_run_returns_success_for_known_command() {
        let runner = build_test_runner();
        let result = run_command_dry_run(&runner, "build");
        assert!(result.is_ok(), "dry_run should succeed for known command");
        assert_eq!(result.unwrap(), ExitCode::SUCCESS);
    }

    #[test]
    fn dry_run_returns_error_for_unknown_command() {
        let runner = build_test_runner();
        let result = run_command_dry_run(&runner, "nonexistent");
        assert!(result.is_err(), "dry_run should fail for unknown command");
    }

    #[test]
    fn dry_run_expands_composite_commands() {
        let runner = build_test_runner();
        let result = run_command_dry_run(&runner, "verify");
        assert!(result.is_ok());
    }

    #[test]
    fn dry_run_shows_program_and_args() {
        let config = TestConfigBuilder::new()
            .exec("echo_test", "echo", &["hello", "world"])
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let mut buf = Vec::new();
        let result = run_command_dry_run_to(&runner, "echo_test", &mut buf);
        assert!(result.is_ok());
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("program: echo"), "got: {output}");
        assert!(output.contains("args:    hello world"), "got: {output}");
    }

    #[test]
    fn dry_run_shows_env_vars() {
        let mut env = std::collections::HashMap::new();
        env.insert("MY_VAR".to_string(), "my_value".to_string());
        let mut spec = ops_core::config::ExecCommandSpec::new("echo", Vec::<String>::new());
        spec.env = env;
        let config = TestConfigBuilder::new()
            .command("with_env", ops_core::config::CommandSpec::Exec(spec))
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let mut buf = Vec::new();
        let result = run_command_dry_run_to(&runner, "with_env", &mut buf);
        assert!(result.is_ok());
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("env:"), "got: {output}");
        assert!(output.contains("MY_VAR=my_value"), "got: {output}");
    }

    #[test]
    fn dry_run_redacts_sensitive_env_vars() {
        let mut env = std::collections::HashMap::new();
        env.insert("API_KEY".to_string(), "secret123".to_string());
        env.insert("PASSWORD".to_string(), "hunter2".to_string());
        let mut spec = ops_core::config::ExecCommandSpec::new("echo", Vec::<String>::new());
        spec.env = env;
        let config = TestConfigBuilder::new()
            .command("with_secrets", ops_core::config::CommandSpec::Exec(spec))
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let mut buf = Vec::new();
        let result = run_command_dry_run_to(&runner, "with_secrets", &mut buf);
        assert!(result.is_ok());
        let output = String::from_utf8(buf).unwrap();
        assert!(
            !output.contains("secret123"),
            "should not leak secret: {output}"
        );
        assert!(
            !output.contains("hunter2"),
            "should not leak password: {output}"
        );
        assert!(
            output.contains("***REDACTED***"),
            "should show redaction: {output}"
        );
    }

    #[test]
    fn dry_run_shows_cwd_if_set() {
        let mut spec = exec_spec("echo", &[]);
        spec.cwd = Some(PathBuf::from("/custom/path"));
        let config = TestConfigBuilder::new()
            .command("with_cwd", ops_core::config::CommandSpec::Exec(spec))
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let mut buf = Vec::new();
        let result = run_command_dry_run_to(&runner, "with_cwd", &mut buf);
        assert!(result.is_ok());
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("cwd:     /custom/path"), "got: {output}");
    }

    #[test]
    fn dry_run_shows_timeout_if_set() {
        let mut spec = exec_spec("echo", &[]);
        spec.timeout_secs = Some(30);
        let config = TestConfigBuilder::new()
            .command("with_timeout", ops_core::config::CommandSpec::Exec(spec))
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let mut buf = Vec::new();
        let result = run_command_dry_run_to(&runner, "with_timeout", &mut buf);
        assert!(result.is_ok());
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("timeout: 30s"), "got: {output}");
    }

    #[test]
    fn dry_run_composite_shows_all_steps() {
        let runner = build_test_runner();
        let mut buf = Vec::new();
        let result = run_command_dry_run_to(&runner, "verify", &mut buf);
        assert!(result.is_ok());
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Command: verify"), "got: {output}");
        assert!(output.contains("Resolved to 2 step(s)"), "got: {output}");
        assert!(output.contains("[1] build"), "got: {output}");
        assert!(output.contains("[2] test"), "got: {output}");
        assert!(output.contains("program: cargo"), "got: {output}");
    }

    #[test]
    fn dry_run_no_args_omits_args_line() {
        let config = TestConfigBuilder::new().exec("simple", "echo", &[]).build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let mut buf = Vec::new();
        run_command_dry_run_to(&runner, "simple", &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("program: echo"), "got: {output}");
        assert!(!output.contains("args:"), "should omit args line: {output}");
    }
}
