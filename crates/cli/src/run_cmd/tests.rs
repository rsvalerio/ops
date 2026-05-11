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
    let config = ops_core::config::Config::empty();
    let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
    assert_eq!(display_cmd_for(&runner, "missing"), "missing");
}

/// READ-7 / TASK-0903: composites now render as their child list rather
/// than the bare id, so plan rows surface a useful label instead of an
/// internal identifier.
#[test]
fn display_cmd_for_composite_returns_child_list() {
    let config = TestConfigBuilder::new()
        .composite("verify", &["build", "test"])
        .build();
    let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
    assert_eq!(display_cmd_for(&runner, "verify"), "build, test");
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

        let result = run_command(
            std::sync::Arc::new(ops_core::config::load_config_or_default("test")),
            "nonexistent",
            RunOptions::default(),
        );
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

        let result = run_command(
            std::sync::Arc::new(ops_core::config::load_config_or_default("test")),
            "echo_test",
            RunOptions::default(),
        );
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

        let result = run_command(
            std::sync::Arc::new(ops_core::config::load_config_or_default("test")),
            "fail_cmd",
            RunOptions::default(),
        );
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

        let result = run_command(
            std::sync::Arc::new(ops_core::config::load_config_or_default("test")),
            "a",
            RunOptions::default(),
        );
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
        let config = ops_core::config::Config::empty();
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

        assert_eq!(display_map.get("verify"), Some(&"build, test".to_string()));
    }

    #[test]
    fn display_cmd_for_with_extension_command() {
        let mut runner = ops_runner::command::CommandRunner::new(
            ops_core::config::Config::empty(),
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
        let result = run_external_command(
            std::sync::Arc::new(ops_core::config::load_config_or_default("test")),
            &args,
            RunOptions::default(),
        );
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
        let result = run_external_command(
            std::sync::Arc::new(ops_core::config::load_config_or_default("test")),
            &args,
            RunOptions {
                dry_run: true,
                ..RunOptions::default()
            },
        );
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
        let result = run_external_command(
            std::sync::Arc::new(ops_core::config::load_config_or_default("test")),
            &args,
            RunOptions {
                dry_run: true,
                ..RunOptions::default()
            },
        );
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), ExitCode::SUCCESS);
    }

    #[test]
    fn run_external_command_single_unknown_errors() {
        let (_dir, _guard) = crate::test_utils::with_temp_config("");
        let args: Vec<OsString> = vec![OsString::from("nonexistent")];
        let result = run_external_command(
            std::sync::Arc::new(ops_core::config::load_config_or_default("test")),
            &args,
            RunOptions::default(),
        );
        assert!(result.is_err());
    }
}

// -- log_step_results --

mod log_step_results_tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct BufWriter(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for BufWriter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> MakeWriter<'a> for BufWriter {
        type Writer = BufWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn capture_debug<F: FnOnce()>(f: F) -> String {
        let buf = BufWriter::default();
        let captured = buf.0.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf)
            .with_max_level(tracing::Level::DEBUG)
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, f);
        let bytes = captured.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap()
    }

    /// TEST-11 / TASK-1302: previously asserted only that the function
    /// didn't panic. Capture the tracing debug events so a mutation that
    /// dropped or swapped any of the per-step fields (id, success,
    /// duration_ms, stdout_len, stderr_len) is caught.
    #[test]
    fn log_step_results_emits_one_debug_event_per_step_with_fields() {
        let results = vec![
            StepResult::success_with_stdout(
                "build",
                std::time::Duration::from_millis(123),
                "hello".to_string(),
            ),
            StepResult::success_with_stdout(
                "test",
                std::time::Duration::from_millis(7),
                "ok".to_string(),
            ),
        ];
        let logs = capture_debug(|| log_step_results(&results));

        assert!(logs.contains("step result"), "got: {logs}");
        assert_eq!(
            logs.matches("step result").count(),
            2,
            "expected one debug event per step result: {logs}"
        );
        assert!(logs.contains("id=build"), "got: {logs}");
        assert!(logs.contains("id=test"), "got: {logs}");
        assert!(logs.contains("duration_ms=123"), "got: {logs}");
        assert!(logs.contains("duration_ms=7"), "got: {logs}");
        assert!(logs.contains("success=true"), "got: {logs}");
        assert!(logs.contains("stdout_len=5"), "got: {logs}");
        assert!(logs.contains("stdout_len=2"), "got: {logs}");
    }

    /// TEST-11 / TASK-1302: an empty slice must emit zero events.
    #[test]
    fn log_step_results_empty_emits_no_events() {
        let logs = capture_debug(|| log_step_results(&[]));
        assert!(
            !logs.contains("step result"),
            "no `step result` event expected for empty slice: {logs}"
        );
    }
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

    /// TEST-11 / TASK-1299: assert against captured dry-run output, not
    /// just `is_ok`. A mutation that emptied `run_command_dry_run_to`
    /// after leaf-id resolution would otherwise still pass these tests.
    #[test]
    fn dry_run_returns_success_for_known_command() {
        let runner = build_test_runner();
        let mut buf = Vec::new();
        let result = run_command_dry_run_to(&runner, "build", &mut buf);
        assert!(result.is_ok(), "dry_run should succeed for known command");
        assert_eq!(result.unwrap(), ExitCode::SUCCESS);
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("program: cargo"),
            "expected resolved program in dry-run output: {output}"
        );
        assert!(
            output.contains("build --all"),
            "expected resolved args in dry-run output: {output}"
        );
    }

    /// TEST-11 / TASK-1299: verify the error message names the missing
    /// command, so a mutation that swallowed the unknown-command label
    /// no longer slips past `is_err()`.
    #[test]
    fn dry_run_returns_error_for_unknown_command() {
        let runner = build_test_runner();
        let result = run_command_dry_run(&runner, "nonexistent");
        let err = result.expect_err("dry_run should fail for unknown command");
        let msg = format!("{err:#}");
        assert!(
            msg.contains("nonexistent"),
            "error must name the missing command: {msg}"
        );
    }

    /// TEST-11 / TASK-1299: the test name claims composites are expanded,
    /// so assert the rendered preview actually lists each leaf step.
    #[test]
    fn dry_run_expands_composite_commands() {
        let runner = build_test_runner();
        let mut buf = Vec::new();
        let result = run_command_dry_run_to(&runner, "verify", &mut buf);
        assert!(result.is_ok());
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("Resolved to 2 step(s)"),
            "expected composite to expand into 2 leaves: {output}"
        );
        assert!(
            output.contains("[1] build"),
            "expected first leaf labelled `build`: {output}"
        );
        assert!(
            output.contains("[2] test"),
            "expected second leaf labelled `test`: {output}"
        );
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

    /// SEC-21 AC #1: an env entry whose name matches a sensitive prefix is
    /// redacted regardless of whether the value trips the heuristic. Uses
    /// values that are deliberately innocuous (short, lowercase-only, no mixed
    /// case) so they would *not* be flagged by `looks_like_secret_value` —
    /// proving the redaction comes from the key match alone.
    #[test]
    fn dry_run_redacts_on_key_match_when_value_is_innocuous() {
        let mut env = std::collections::HashMap::new();
        // Each value is short / lowercase-only and would not be flagged by
        // any value heuristic on its own.
        env.insert("MY_AUTH_HEADER".to_string(), "ok".to_string());
        env.insert("USER_TOKEN".to_string(), "x".to_string());
        env.insert("DEPLOY_PASSWORD".to_string(), "a".to_string());
        env.insert("APP_API_KEY".to_string(), "z".to_string());
        let mut spec = ops_core::config::ExecCommandSpec::new("echo", Vec::<String>::new());
        spec.env = env;
        let config = TestConfigBuilder::new()
            .command(
                "with_named_secrets",
                ops_core::config::CommandSpec::Exec(spec),
            )
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let mut buf = Vec::new();
        let result = run_command_dry_run_to(&runner, "with_named_secrets", &mut buf);
        assert!(result.is_ok());
        let output = String::from_utf8(buf).unwrap();
        for k in [
            "MY_AUTH_HEADER",
            "USER_TOKEN",
            "DEPLOY_PASSWORD",
            "APP_API_KEY",
        ] {
            assert!(
                output.contains(&format!("{k}=***REDACTED***")),
                "key {k} should be redacted via name match alone, got: {output}"
            );
        }
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

    /// ERR-7 (TASK-0576): when an env var referenced in argv is non-UTF-8,
    /// dry-run must surface the failure to the user via a returned error
    /// rather than silently rendering the literal `${VAR}` and only logging.
    #[cfg(unix)]
    #[test]
    #[serial_test::serial]
    fn dry_run_surfaces_non_utf8_env_var_in_args() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let key = "OPS_TEST_DRY_RUN_NON_UTF8";
        let bad: OsString = OsString::from_vec(vec![0xff, 0xfe]);
        // SAFETY: serialised by #[serial_test::serial] to avoid env races.
        unsafe { std::env::set_var(key, &bad) };

        let config = TestConfigBuilder::new()
            .exec("leaks_env", "echo", &[format!("${key}").as_str()])
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let mut buf = Vec::new();
        let result = run_command_dry_run_to(&runner, "leaks_env", &mut buf);
        // SAFETY: serialised by #[serial_test::serial].
        unsafe { std::env::remove_var(key) };
        let err = result.expect_err("dry-run must propagate non-UTF-8 env failure");
        let msg = format!("{err:#}");
        assert!(
            msg.contains(key),
            "error must name the offending var: {msg}"
        );
    }

    /// READ-5 (TASK-0543): a non-UTF-8 cwd PathBuf is rendered through a
    /// lossy conversion in the dry-run preview. Annotate explicitly so the
    /// user can tell the printed path is approximate, not byte-exact.
    #[cfg(unix)]
    #[test]
    fn dry_run_annotates_non_utf8_cwd() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let bad: OsString = OsString::from_vec(b"/tmp/\xff\xfe".to_vec());
        let mut spec = exec_spec("echo", &[]);
        spec.cwd = Some(PathBuf::from(bad));
        let config = TestConfigBuilder::new()
            .command("non_utf8_cwd", ops_core::config::CommandSpec::Exec(spec))
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let mut buf = Vec::new();
        run_command_dry_run_to(&runner, "non_utf8_cwd", &mut buf).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("(non-UTF-8 path; lossy preview)"),
            "annotation missing: {output}"
        );
    }

    /// SEC-21 / TASK-1184: print_exec_spec routes program / args / env
    /// values / cwd through `sanitise_line` so an adversarial `.ops.toml`
    /// (or `${VAR}` expansion) carrying ANSI clear-screen / cursor-move
    /// bytes cannot repaint the operator's terminal during dry-run.
    /// Mirrors the stderr policy from `ops_core::ui::emit_to`.
    #[test]
    fn dry_run_escapes_ansi_in_program_args_env_and_cwd() {
        let mut spec = ops_core::config::ExecCommandSpec::new(
            "x\u{1b}[2J\u{1b}[H",
            vec!["arg\u{1b}[31m".to_string()],
        );
        let mut env = std::collections::HashMap::new();
        env.insert("INNOCUOUS".to_string(), "v\u{1b}[2K".to_string());
        spec.env = env;
        spec.cwd = Some(PathBuf::from("/tmp/cwd\u{1b}[2J"));
        let config = TestConfigBuilder::new()
            .command("evil", ops_core::config::CommandSpec::Exec(spec))
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let mut buf = Vec::new();
        let result = run_command_dry_run_to(&runner, "evil", &mut buf);
        assert!(result.is_ok());
        let output = String::from_utf8(buf).unwrap();
        assert!(
            !output.contains('\u{1b}'),
            "raw ESC byte must not survive in dry-run audit output: {output:?}"
        );
        assert!(
            output.contains("\\x1b"),
            "expected escaped \\x1b form in: {output:?}"
        );
    }

    /// SEC-21 / TASK-1275: the command name and resolved leaf id printed
    /// by `run_command_dry_run_to` come from `.ops.toml` keys (which TOML
    /// allows to be arbitrary Unicode). They must be routed through
    /// `audit_safe` like program/args/env/cwd so an adversarial config
    /// key like `"evil\x1b[2J"` cannot repaint the operator's terminal.
    #[test]
    fn dry_run_escapes_ansi_in_command_name_and_id() {
        let name = "evil\u{1b}[2J";
        let spec = ops_core::config::ExecCommandSpec::new("true", Vec::<String>::new());
        let config = TestConfigBuilder::new()
            .command(name, ops_core::config::CommandSpec::Exec(spec))
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, PathBuf::from("."));
        let mut buf = Vec::new();
        run_command_dry_run_to(&runner, name, &mut buf).expect("dry-run");
        let output = String::from_utf8(buf).unwrap();
        assert!(
            !output.contains('\u{1b}'),
            "raw ESC byte must not survive in dry-run name/id: {output:?}"
        );
        assert!(
            output.contains("\\x1b"),
            "expected escaped \\x1b form for name/id in: {output:?}"
        );
    }
}

mod raw_warnings_tests {
    use crate::run_cmd::emit_raw_warnings;
    use std::sync::{Arc, Mutex};
    use tracing_subscriber::fmt::MakeWriter;

    #[derive(Clone, Default)]
    struct BufWriter(Arc<Mutex<Vec<u8>>>);
    impl std::io::Write for BufWriter {
        fn write(&mut self, b: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(b);
            Ok(b.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    impl<'a> MakeWriter<'a> for BufWriter {
        type Writer = BufWriter;
        fn make_writer(&'a self) -> Self::Writer {
            self.clone()
        }
    }

    fn capture<F: FnOnce()>(f: F) -> String {
        let buf = BufWriter::default();
        let captured = buf.0.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_writer(buf)
            .with_max_level(tracing::Level::WARN)
            .with_ansi(false)
            .finish();
        tracing::subscriber::with_default(subscriber, f);
        let bytes = captured.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap()
    }

    #[test]
    fn raw_with_tap_emits_warning() {
        let logs = capture(|| emit_raw_warnings(false, true));
        assert!(logs.contains("--tap is ignored under --raw"), "got: {logs}");
    }

    #[test]
    fn raw_with_parallel_emits_warning() {
        let logs = capture(|| emit_raw_warnings(true, false));
        assert!(
            logs.contains("--raw forces sequential execution"),
            "got: {logs}"
        );
    }

    #[test]
    fn raw_with_both_emits_two_warnings() {
        let logs = capture(|| emit_raw_warnings(true, true));
        assert!(logs.contains("--tap is ignored"), "got: {logs}");
        assert!(logs.contains("--raw forces sequential"), "got: {logs}");
    }

    #[test]
    fn raw_clean_emits_nothing() {
        let logs = capture(|| emit_raw_warnings(false, false));
        assert!(logs.is_empty(), "unexpected output: {logs}");
    }

    /// CL-5 / TASK-0755: the single-command `--raw` path used to inline its
    /// own copy of the tap-warning string; route it through
    /// `emit_raw_warnings` so the message lives in one place and never fires
    /// twice for the same invocation.
    #[test]
    fn single_command_raw_path_emits_tap_warning_exactly_once() {
        use crate::run_cmd::run_command_raw;
        use crate::test_utils::TestConfigBuilder;

        let config = TestConfigBuilder::new()
            .exec("echo_one", "echo", &["hi"])
            .build();
        let runner = ops_runner::command::CommandRunner::new(config, std::path::PathBuf::from("."));
        let logs = capture(|| {
            let _ = run_command_raw(&runner, "echo_one", true);
        });
        let tap_warnings = logs.matches("--tap is ignored").count();
        assert_eq!(
            tap_warnings, 1,
            "single-command raw path with --tap must emit the warning exactly once; got: {logs}"
        );
    }
}

mod nested_parallel_detection_tests {
    use crate::run_cmd::composite_tree_has_parallel;
    use crate::test_utils::TestConfigBuilder;
    use ops_core::config::{CommandSpec, CompositeCommandSpec};
    use std::path::PathBuf;

    fn runner_with(config: ops_core::config::Config) -> ops_runner::command::CommandRunner {
        ops_runner::command::CommandRunner::new(config, PathBuf::from("."))
    }

    #[test]
    fn detects_top_level_parallel() {
        let mut comp = CompositeCommandSpec::new(["a", "b"]);
        comp.parallel = true;
        let mut config = TestConfigBuilder::new()
            .exec("a", "echo", &["a"])
            .exec("b", "echo", &["b"])
            .build();
        config
            .commands
            .insert("top".to_string(), CommandSpec::Composite(comp));
        assert!(composite_tree_has_parallel(&runner_with(config), "top"));
    }

    #[test]
    fn detects_nested_parallel() {
        let mut inner = CompositeCommandSpec::new(["a", "b"]);
        inner.parallel = true;
        let outer = CompositeCommandSpec::new(["inner", "tail"]); // sequential
        let mut config = TestConfigBuilder::new()
            .exec("a", "echo", &["a"])
            .exec("b", "echo", &["b"])
            .exec("tail", "echo", &["t"])
            .build();
        config
            .commands
            .insert("inner".to_string(), CommandSpec::Composite(inner));
        config
            .commands
            .insert("outer".to_string(), CommandSpec::Composite(outer));
        assert!(
            composite_tree_has_parallel(&runner_with(config), "outer"),
            "nested parallel composite should be detected"
        );
    }

    #[test]
    fn no_parallel_returns_false() {
        let outer = CompositeCommandSpec::new(["a", "b"]);
        let mut config = TestConfigBuilder::new()
            .exec("a", "echo", &["a"])
            .exec("b", "echo", &["b"])
            .build();
        config
            .commands
            .insert("outer".to_string(), CommandSpec::Composite(outer));
        assert!(!composite_tree_has_parallel(&runner_with(config), "outer"));
    }

    #[test]
    fn leaf_command_returns_false() {
        let config = TestConfigBuilder::new().exec("a", "echo", &["a"]).build();
        assert!(!composite_tree_has_parallel(&runner_with(config), "a"));
    }

    #[test]
    fn handles_cycles_without_panicking() {
        // Two composites referencing each other — should terminate.
        let a = CompositeCommandSpec::new(["b"]);
        let b = CompositeCommandSpec::new(["a"]);
        let mut config = ops_core::config::Config::empty();
        config
            .commands
            .insert("a".to_string(), CommandSpec::Composite(a));
        config
            .commands
            .insert("b".to_string(), CommandSpec::Composite(b));
        assert!(!composite_tree_has_parallel(&runner_with(config), "a"));
    }
}

/// PATTERN-1 / TASK-0754: `merge_plan` aggregates `parallel` / `fail_fast`
/// flags by walking the composite tree, so an outer composite that wraps a
/// parallel inner composite still surfaces `any_parallel = true` and a
/// nested `fail_fast = false` propagates upward.
mod merge_plan_nested_aggregation_tests {
    use crate::run_cmd::plan::merge_plan;
    use crate::test_utils::TestConfigBuilder;
    use ops_core::config::{CommandSpec, CompositeCommandSpec, Config};
    use std::path::PathBuf;

    fn runner_with(config: Config) -> ops_runner::command::CommandRunner {
        ops_runner::command::CommandRunner::new(config, PathBuf::from("."))
    }

    #[test]
    fn merge_plan_picks_up_nested_parallel() {
        let mut inner = CompositeCommandSpec::new(["a", "b"]);
        inner.parallel = true;
        let outer = CompositeCommandSpec::new(["inner"]); // outer.parallel = false
        let mut config = TestConfigBuilder::new()
            .exec("a", "echo", &["a"])
            .exec("b", "echo", &["b"])
            .build();
        config
            .commands
            .insert("inner".to_string(), CommandSpec::Composite(inner));
        config
            .commands
            .insert("outer".to_string(), CommandSpec::Composite(outer));

        let (_, any_parallel, fail_fast) = merge_plan(&runner_with(config), &["outer"]).unwrap();
        assert!(
            any_parallel,
            "nested inner.parallel must propagate to merge_plan"
        );
        assert!(fail_fast, "no composite disables fail_fast → defaults true");
    }

    #[test]
    fn merge_plan_picks_up_nested_fail_fast_disabled() {
        let mut inner = CompositeCommandSpec::new(["a"]);
        inner.fail_fast = false;
        let outer = CompositeCommandSpec::new(["inner"]); // outer.fail_fast defaults true
        let mut config = TestConfigBuilder::new().exec("a", "echo", &["a"]).build();
        config
            .commands
            .insert("inner".to_string(), CommandSpec::Composite(inner));
        config
            .commands
            .insert("outer".to_string(), CommandSpec::Composite(outer));

        let (_, _any_parallel, fail_fast) = merge_plan(&runner_with(config), &["outer"]).unwrap();
        assert!(
            !fail_fast,
            "nested inner.fail_fast = false must propagate to merge_plan"
        );
    }

    /// PATTERN-1 / TASK-1091: `merge_plan` rejects an empty `names` slice
    /// rather than returning `(empty_plan, false, true)` and letting the
    /// executor run zero steps under a silent success. The single
    /// production caller (`run_external_command`) already rejects empty
    /// argv earlier; this test pins the defensive fail-loud contract so a
    /// future refactor cannot regress to the silent-success shape.
    #[test]
    fn merge_plan_rejects_empty_names() {
        let config = TestConfigBuilder::new().exec("a", "echo", &["a"]).build();
        let err = merge_plan(&runner_with(config), &[]).unwrap_err();
        let msg = format!("{err}");
        assert!(
            msg.contains("empty names slice"),
            "error must name the empty-slice contract, got: {msg}"
        );
    }
}

/// ERR-1 / TASK-1234: dry-run overrides --raw and --tap. The execute path's
/// `emit_raw_warnings` already surfaces the same flag conflicts; pin the
/// dry-run-side contract via the static-message helper so a future
/// refactor cannot silently drop one branch.
mod dry_run_override_warnings_tests {
    #[test]
    fn dry_run_overrides_messages_emits_for_raw() {
        let msgs = super::super::dry_run_overrides_messages(true, false);
        assert_eq!(msgs.len(), 1, "raw alone must produce exactly one message");
        assert!(
            msgs[0].contains("--raw is ignored under --dry-run"),
            "raw override message missing: got {msgs:?}"
        );
    }

    #[test]
    fn dry_run_overrides_messages_emits_for_tap() {
        let msgs = super::super::dry_run_overrides_messages(false, true);
        assert_eq!(msgs.len(), 1, "tap alone must produce exactly one message");
        assert!(
            msgs[0].contains("--tap is ignored under --dry-run"),
            "tap override message missing: got {msgs:?}"
        );
    }

    #[test]
    fn dry_run_overrides_messages_emits_for_both() {
        let msgs = super::super::dry_run_overrides_messages(true, true);
        assert_eq!(msgs.len(), 2, "both flags must produce both messages");
        assert!(msgs.iter().any(|m| m.contains("--raw is ignored")));
        assert!(msgs.iter().any(|m| m.contains("--tap is ignored")));
    }

    #[test]
    fn dry_run_overrides_messages_silent_when_no_conflict() {
        let msgs = super::super::dry_run_overrides_messages(false, false);
        assert!(
            msgs.is_empty(),
            "no override flags must not emit any message: got {msgs:?}"
        );
    }
}
