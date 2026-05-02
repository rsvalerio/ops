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
            &ops_core::config::load_config_or_default("test"),
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
            &ops_core::config::load_config_or_default("test"),
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
            &ops_core::config::load_config_or_default("test"),
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
            &ops_core::config::load_config_or_default("test"),
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

        assert_eq!(display_map.get("verify"), Some(&"build, test".to_string()));
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
        let result = run_external_command(
            &ops_core::config::load_config_or_default("test"),
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
            &ops_core::config::load_config_or_default("test"),
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
            &ops_core::config::load_config_or_default("test"),
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
            &ops_core::config::load_config_or_default("test"),
            &args,
            RunOptions::default(),
        );
        assert!(result.is_err());
    }
}

// -- log_step_results --

#[test]
fn log_step_results_does_not_panic() {
    let results = vec![StepResult::success_with_stdout(
        "test",
        std::time::Duration::from_millis(100),
        "output".to_string(),
    )];
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
        let mut config = ops_core::config::Config::default();
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
}
