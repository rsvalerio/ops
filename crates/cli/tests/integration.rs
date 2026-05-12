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
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
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

// TEST-18 (TASK-0957): pin HOME / XDG_CONFIG_HOME to a process-wide empty
// tempdir so a developer's `~/.ops.toml` or `~/.config/ops/...` cannot leak
// into integration runs. The directory is held in a `OnceLock` so it lives
// for the whole test binary; tests that depend on user-config absence rely
// on this precondition (see TEST-18 AC#3 documentation note below).
fn isolated_home() -> &'static Path {
    static HOME: OnceLock<PathBuf> = OnceLock::new();
    HOME.get_or_init(|| {
        let dir = tempfile::Builder::new()
            .prefix("ops-isolated-home-")
            .tempdir()
            .expect("isolated home tempdir");
        // Intentionally leak the TempDir handle: directory lifetime equals
        // the test binary's lifetime, and OS tmp cleanup reclaims it.
        dir.keep()
    })
}

// TEST-18 (TASK-0957): every test that spawns `ops` must go through this
// helper. Tests assuming "no user config" inherit isolation from `HOME` /
// `XDG_CONFIG_HOME` being pinned to an empty dir and from `OPS_*` env vars
// being cleared, so behaviour matches CI regardless of the developer's
// machine state.
#[allow(deprecated)]
fn ops() -> Command {
    let mut cmd = Command::cargo_bin("ops").expect("ops binary");
    let home = isolated_home();
    // AC#1: redirect user-config search roots to a known empty dir.
    cmd.env("HOME", home);
    cmd.env("XDG_CONFIG_HOME", home);
    // AC#2: drop ambient OPS_* configuration so the outer shell cannot
    // alter test behaviour.
    for (key, _) in std::env::vars_os() {
        if key.to_string_lossy().starts_with("OPS_") {
            cmd.env_remove(key);
        }
    }
    cmd
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
    // TEST-11 (TASK-0954): assert the specific failure mode (unknown
    // command) so a regression that fails for an unrelated reason — e.g.
    // missing .ops.toml or a panic — does not silently pass this test.
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
                .failure()
                .stderr(predicate::str::contains(
                    "unknown command: nonexistent_command",
                ));
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

// TEST-25 (TASK-0955): verify parallel scheduling via a side-channel
// rendezvous. The previous version asserted only `success()` + " in " —
// identical to the sequential composite test — so a regression that
// silently turned `parallel = true` into sequential execution still
// passed. Each child writes its own marker file and then waits for the
// other child's marker. Under sequential scheduling the first child
// blocks waiting for a marker the second child has not yet had a chance
// to create, the wait loop times out non-zero, and the composite fails.
// Under parallel scheduling both markers appear quickly and both
// children exit cleanly. Unix-only because the helper relies on `sh`.
#[cfg(unix)]
#[test]
fn cli_run_parallel_composite_command_runs_concurrently() {
    let dir = temp_dir();
    let marker_dir = dir.path().join("markers");
    std::fs::create_dir_all(&marker_dir).expect("create marker dir");
    let a = marker_dir.join("a");
    let b = marker_dir.join("b");

    // ~5s upper bound on the wait loop (100 * 50ms). Sequential runs hit
    // this bound and fail; parallel runs typically resolve in <100ms.
    let waiter = |self_marker: &Path, peer_marker: &Path| {
        format!(
            "touch {self_q} && for _ in $(seq 1 100); do \
             [ -f {peer_q} ] && exit 0; sleep 0.05; done; \
             echo 'parallel rendezvous timed out' >&2; exit 1",
            self_q = shell_quote(self_marker),
            peer_q = shell_quote(peer_marker),
        )
    };

    write_ops_toml(
        dir.path(),
        &format!(
            r#"
[commands.echo_a]
program = "sh"
args = ["-c", {script_a}]

[commands.echo_b]
program = "sh"
args = ["-c", {script_b}]

[commands.par]
commands = ["echo_a", "echo_b"]
parallel = true
"#,
            script_a = toml_string(&waiter(&a, &b)),
            script_b = toml_string(&waiter(&b, &a)),
        ),
    );

    ops()
        .arg("par")
        .current_dir(dir.path())
        .assert()
        .success()
        .stderr(predicate::str::contains(" in "));

    assert!(a.exists(), "marker a not written");
    assert!(b.exists(), "marker b not written");
}

#[cfg(unix)]
fn shell_quote(p: &Path) -> String {
    // Single-quote-wrap, escaping embedded single quotes as '\''.
    let s = p.to_string_lossy().replace('\'', r"'\''");
    format!("'{s}'")
}

#[cfg(unix)]
fn toml_string(s: &str) -> String {
    // Minimal basic-string escaper sufficient for the shell snippets
    // above (no control chars, no unicode escapes needed).
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

// -- TQ-003/TQ-018: Timeout handling at CLI level --
// TEST-15 (TASK-0953): use a process that never terminates on its own so
// timeout firing is the only way the child can exit. The previous version
// raced a 3s sleep against a 1s timeout — small ratio, sleep-based, prone
// to flake on slow CI. `tail -f /dev/null` blocks indefinitely without
// depending on stdin or the host scheduler. Windows still uses `ping`
// with a large count (>>timeout): the ratio is so wide that timeout is
// the only feasible termination cause.

#[test]
fn cli_run_command_with_timeout() {
    let dir = temp_dir();
    let blocker = if cfg!(windows) {
        r#"program = "ping"
args = ["-n", "999", "127.0.0.1"]"#
    } else {
        r#"program = "tail"
args = ["-f", "/dev/null"]"#
    };
    write_ops_toml(
        dir.path(),
        &format!(
            r#"
[commands.slow_cmd]
{}
timeout_secs = 1
"#,
            blocker
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
    // TEST-11 (TASK-0954): assert the failure cites the .ops.toml parse
    // error, not just any non-zero exit, so a regression that fails for
    // an unrelated reason cannot pass this test.
    with_ops_toml(
        r#"
[commands.broken
program = "echo"
"#,
        |path| {
            ops()
                .arg("broken")
                .current_dir(path)
                .assert()
                .failure()
                .stderr(predicate::str::contains(
                    "failed to parse config file: .ops.toml",
                ));
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
                .stdout(predicate::str::contains("visible"))
                // TEST-12 (TASK-1081): the raw secret must never reach
                // stdout or stderr, even alongside the redacted line.
                .stdout(predicate::str::contains("super_secret_value").not())
                .stderr(predicate::str::contains("super_secret_value").not());
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

/// TEST-11 / TASK-1367: pin a stable about-card marker rather than the
/// near-tautological binary name "ops" (which also appears in help, errors,
/// and version banners on any successful run). The `project` field label is
/// part of the rendered about-card contract — a regression where the about
/// header silently fails to render would no longer satisfy this assertion.
#[test]
#[cfg_attr(not(feature = "stack-rust"), ignore)]
fn cli_about_shows_header() {
    let dir = temp_dir();
    write_ops_toml(
        dir.path(),
        r#"
[output]
theme = "classic"

[about]
fields = ["project"]
"#,
    );
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "demo"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("write Cargo.toml");
    ops()
        .arg("about")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("project"))
        .stdout(predicate::str::contains("demo"));
}

/// TEST-25 / TASK-1364: assert that `about --refresh` actually exercises the
/// live about-render path (project_identity provider + card render). The
/// previous assertion-only-on-`.success()` shape would have passed a
/// regression that silently ignored `--refresh` and emitted nothing; pin
/// the rendered card content via the same stable `project` marker that
/// `cli_about_shows_header` uses, and seed `.ops.toml` with `[about].fields`
/// so the field-filter does not erase the marker.
#[test]
#[cfg_attr(not(feature = "stack-rust"), ignore)]
fn cli_about_refresh_flag() {
    let dir = temp_dir();
    write_ops_toml(
        dir.path(),
        r#"
[output]
theme = "classic"

[about]
fields = ["project"]
"#,
    );
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "demo"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("write Cargo.toml");
    ops()
        .arg("about")
        .arg("--refresh")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("project"))
        .stdout(predicate::str::contains("demo"));
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

    // TEST-11 (TASK-0954): assert the failure cites the offending
    // .ops.d/invalid.toml file, not just any non-zero exit.
    ops()
        .arg("build")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "failed to parse config file: .ops.d/invalid.toml",
        ));
}
