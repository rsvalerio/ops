//! Integration tests for ops CLI.
//!
//! # Architecture (CQ-025)
//!
//! This file tests CLI behavior through `assert_cmd`, spawning actual processes.
//! Tests are grouped by:
//!
//! - **Version/Help**: `cli_version`, `cli_help`
//! - **Init Command**: `cli_init_*`
//! - **Run Command**: `cli_run_*`, timeout, composite
//! - **Theme Command**: `cli_theme_*`
//!
//! The helper functions (`with_ops_toml`, `temp_dir`, etc.) reduce boilerplate
//! for the common pattern of creating a temp directory with a config file.
//!
//! DUP-010: These helpers could be extracted to a shared module, but are kept
//! inline because:
//! 1. Integration tests are in `tests/` and don't share code with `src/`
//! 2. The helpers are 3-10 lines each
//! 3. Each test file having its own helpers is idiomatic for Rust integration tests

use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;
use tempfile::TempDir;

fn with_ops_toml(content: &str, f: impl FnOnce(&Path)) {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join(".ops.toml"), content).expect("write .ops.toml");
    f(dir.path());
}

fn temp_dir() -> TempDir {
    tempfile::tempdir().expect("tempdir")
}

fn write_ops_toml(dir: &Path, content: &str) {
    std::fs::write(dir.join(".ops.toml"), content).expect("write .ops.toml");
}

fn read_ops_toml(dir: &Path) -> String {
    std::fs::read_to_string(dir.join(".ops.toml")).expect("read .ops.toml")
}

#[allow(deprecated)]
fn ops() -> Command {
    Command::cargo_bin("ops").expect("ops binary")
}

#[test]
fn cli_version() {
    ops()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("ops"))
        .stdout(predicate::function(|s: &str| {
            s.contains("ops") && s.chars().any(|c| c.is_ascii_digit())
        }));
}

#[test]
fn cli_help() {
    ops()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Batteries-included"))
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Commands:"));
}

#[test]
fn cli_init_creates_ops_toml() {
    let dir = temp_dir();
    ops().arg("init").current_dir(dir.path()).assert().success();

    assert!(dir.path().join(".ops.toml").exists());
}

#[test]
fn cli_init_no_overwrite_without_force() {
    let dir = temp_dir();
    write_ops_toml(dir.path(), "existing content");

    ops().arg("init").current_dir(dir.path()).assert().success();

    assert_eq!(read_ops_toml(dir.path()), "existing content");
}

#[test]
fn cli_init_force_overwrites() {
    let dir = temp_dir();
    write_ops_toml(dir.path(), "existing content");

    ops()
        .arg("init")
        .arg("--force")
        .current_dir(dir.path())
        .assert()
        .success();

    assert!(read_ops_toml(dir.path()).contains("[output]"));
}

#[test]
fn cli_init_in_rust_project_with_commands_flag_writes_stack_commands() {
    let dir = temp_dir();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        "[package]\nname = \"x\"\nversion = \"0.1.0\"\n",
    )
    .expect("write Cargo.toml");

    ops()
        .args(["init", "--commands"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = read_ops_toml(dir.path());
    assert!(
        content.contains("[commands.build]"),
        "init --commands in Rust project must write [commands.build]"
    );
    assert!(
        content.contains("[commands.clippy]"),
        "init --commands in Rust project must write [commands.clippy]"
    );
    assert!(
        content.contains("[commands.verify]"),
        "init --commands in Rust project must write [commands.verify]"
    );
}

#[test]
fn cli_init_default_writes_minimal_output_only() {
    let dir = temp_dir();

    ops().arg("init").current_dir(dir.path()).assert().success();

    let content = read_ops_toml(dir.path());
    assert!(
        content.contains("[output]"),
        "init must write base [output]"
    );
    // Default init (no flags) should not include themes or commands.
    assert!(
        !content.contains("[themes.classic]"),
        "default init should not include theme definitions"
    );
    assert!(
        !content.contains("[commands.build]"),
        "default init should not add stack commands"
    );
}

#[test]
fn cli_init_with_themes_flag_includes_themes() {
    let dir = temp_dir();

    ops()
        .args(["init", "--themes"])
        .current_dir(dir.path())
        .assert()
        .success();

    let content = read_ops_toml(dir.path());
    assert!(
        content.contains("[themes.classic]"),
        "init --themes must include classic theme"
    );
    assert!(
        content.contains("[themes.compact]"),
        "init --themes must include compact theme"
    );
}

#[test]
fn cli_run_unknown_command_fails() {
    with_ops_toml(
        r#"
[commands.echo_test]
program = "echo"
args = ["hello"]
"#,
        |path| {
            ops()
                .arg("nonexistent_command")
                .current_dir(path)
                .assert()
                .failure();
        },
    );
}

#[test]
fn cli_run_echo_success() {
    with_ops_toml(
        r#"
[commands.echo_test]
program = "echo"
args = ["integration_test_output"]
"#,
        |path| {
            ops()
                .arg("echo_test")
                .current_dir(path)
                .assert()
                .success()
                .stderr(predicate::str::contains(" in "));
        },
    );
}

#[test]
fn cli_run_failing_command() {
    let dir = temp_dir();
    let program = if cfg!(windows) { "cmd" } else { "false" };
    let args: Vec<&str> = if cfg!(windows) {
        vec!["/C", "exit", "1"]
    } else {
        vec![]
    };

    let args_toml = if args.is_empty() {
        String::new()
    } else {
        format!(
            "args = [{}]",
            args.iter()
                .map(|a| format!("\"{}\"", a))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    write_ops_toml(
        dir.path(),
        &format!(
            r#"
[commands.fail_cmd]
program = "{}"
{}
"#,
            program, args_toml
        ),
    );

    ops()
        .arg("fail_cmd")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(" in "));
}

#[test]
fn cli_run_composite_command() {
    with_ops_toml(
        r#"
[commands.echo_a]
program = "echo"
args = ["a"]

[commands.echo_b]
program = "echo"
args = ["b"]

[commands.both]
commands = ["echo_a", "echo_b"]
"#,
        |path| {
            ops()
                .arg("both")
                .current_dir(path)
                .assert()
                .success()
                .stderr(predicate::str::contains(" in "));
        },
    );
}

// -- TQ-017: Parallel composite commands --

#[test]
fn cli_run_parallel_composite_command() {
    with_ops_toml(
        r#"
[commands.echo_a]
program = "echo"
args = ["a"]

[commands.echo_b]
program = "echo"
args = ["b"]

[commands.par]
commands = ["echo_a", "echo_b"]
parallel = true
"#,
        |path| {
            ops()
                .arg("par")
                .current_dir(path)
                .assert()
                .success()
                .stderr(predicate::str::contains(" in "));
        },
    );
}

// -- TQ-003/TQ-018: Timeout handling at CLI level --
// Uses 3s sleep with 1s timeout for reliable timing under CI load (2x safety margin)

#[test]
fn cli_run_command_with_timeout() {
    let dir = temp_dir();
    let sleep_cmd = if cfg!(windows) {
        r#"program = "ping"
args = ["-n", "4", "127.0.0.1"]"#
    } else {
        r#"program = "sleep"
args = ["3"]"#
    };
    write_ops_toml(
        dir.path(),
        &format!(
            r#"
[commands.slow_cmd]
{}
timeout_secs = 1
"#,
            sleep_cmd
        ),
    );

    ops()
        .arg("slow_cmd")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(" in "));
}

// -- TQ-019: Malformed TOML config error handling --

#[test]
fn cli_run_with_malformed_toml() {
    with_ops_toml(
        r#"
[commands.broken
program = "echo"
"#,
        |path| {
            ops().arg("broken").current_dir(path).assert().failure();
        },
    );
}

// -- TASK-0068: run-before-commit/push with malformed config surfaces error --

#[test]
fn cli_run_before_commit_with_malformed_toml_fails() {
    with_ops_toml(
        r#"
[commands.broken
program = "echo"
"#,
        |path| {
            ops()
                .arg("run-before-commit")
                .current_dir(path)
                .assert()
                .failure();
        },
    );
}

// -- TQ-017: Theme list command --

#[test]
fn cli_theme_list() {
    let dir = temp_dir();
    write_ops_toml(
        dir.path(),
        r#"[output]
theme = "classic"
"#,
    );

    ops()
        .arg("theme")
        .arg("list")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("classic"));
}

#[test]
fn cli_theme_list_with_custom_theme() {
    let dir = temp_dir();
    write_ops_toml(
        dir.path(),
        r#"
[output]
theme = "classic"

[themes.my-custom]
icon_pending = "○"
icon_running = ""
icon_succeeded = "●"
icon_failed = "✗"
icon_skipped = "—"
separator_char = '.'
step_indent = "  "
running_template = "  {spinner:.cyan}{msg} {elapsed:.dim}"
tick_chars = "⠁⠂⠄ "
running_template_overhead = 7
plan_header_style = "plain"
summary_prefix = "→ "
summary_separator = ""
"#,
    );

    ops()
        .arg("theme")
        .arg("list")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("my-custom"));
}

// -- TQ-002/003: Dry-run output verification --

#[test]
fn cli_dry_run_shows_program_and_args() {
    with_ops_toml(
        r#"
[commands.build]
program = "cargo"
args = ["build", "--release"]
"#,
        |path| {
            ops()
                .arg("--dry-run")
                .arg("build")
                .current_dir(path)
                .assert()
                .success()
                .stdout(predicate::str::contains("cargo"))
                .stdout(predicate::str::contains("build --release"));
        },
    );
}

#[test]
fn cli_dry_run_redacts_sensitive_env() {
    with_ops_toml(
        r#"
[commands.secret_cmd]
program = "echo"
args = ["hello"]

[commands.secret_cmd.env]
API_KEY = "super_secret_value"
NORMAL_VAR = "visible"
"#,
        |path| {
            ops()
                .arg("--dry-run")
                .arg("secret_cmd")
                .current_dir(path)
                .assert()
                .success()
                .stdout(predicate::str::contains("***REDACTED***"))
                .stdout(predicate::str::contains("visible"));
        },
    );
}

#[test]
fn cli_dry_run_shows_timeout() {
    with_ops_toml(
        r#"
[commands.slow]
program = "sleep"
args = ["10"]
timeout_secs = 5
"#,
        |path| {
            ops()
                .arg("--dry-run")
                .arg("slow")
                .current_dir(path)
                .assert()
                .success()
                .stdout(predicate::str::contains("timeout: 5s"));
        },
    );
}

// -- About smoke tests (require stack-rust; run with --features stack-rust) --

#[test]
#[cfg_attr(not(feature = "stack-rust"), ignore)]
fn cli_about_shows_header() {
    ops()
        .arg("about")
        .assert()
        .success()
        .stdout(predicate::str::contains("ops"));
}

#[test]
#[cfg_attr(not(feature = "stack-rust"), ignore)]
fn cli_about_refresh_flag() {
    ops().arg("about").arg("--refresh").assert().success();
}

// -- TQ-017: Malformed .ops.d/ handling --

#[test]
fn cli_with_invalid_ops_d_file() {
    let dir = temp_dir();
    write_ops_toml(
        dir.path(),
        r#"[output]
theme = "classic"
"#,
    );

    let ops_d = dir.path().join(".ops.d");
    std::fs::create_dir_all(&ops_d).expect("create .ops.d");
    std::fs::write(ops_d.join("invalid.toml"), "not valid toml [[[[").expect("write invalid");

    ops()
        .arg("build")
        .current_dir(dir.path())
        .assert()
        .failure();
}
