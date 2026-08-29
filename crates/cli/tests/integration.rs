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

// docs/clippy.md layer 2: an integration-test target is its own crate, so the
// library crate roots' `#![cfg_attr(test, allow(..))]` block does not reach it
// and `allow-expect-in-tests` only covers `#[test]` bodies. The helpers below
// are test scaffolding where a panic on a broken fixture is the desired
// failure mode.
#![allow(clippy::expect_used)]
// The `{spinner}` / `{msg}` / `{elapsed}` tokens in the theme fixtures below are
// `indicatif` template placeholders, not Rust format arguments.
#![allow(clippy::literal_string_with_formatting_args)]

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
fn cli_run_echo_reports_resolved_command_and_timing_in_stderr() {
    // The runner consumes the child's stdout, so we cannot observe the
    // echoed payload here. Instead pin the user-visible status output:
    // the resolved command line (program + arg) and the timing suffix
    // appear on stderr, which is what would regress if the runner were
    // wired to a no-op driver or the wrong program resolved.
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
                .stderr(predicate::str::contains("echo integration_test_output"))
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
                .map(|a| format!("\"{a}\""))
                .collect::<Vec<_>>()
                .join(", ")
        )
    };

    write_ops_toml(
        dir.path(),
        &format!(
            r#"
[commands.fail_cmd]
program = "{program}"
{args_toml}
"#
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
    let mut out = String::with_capacity(s.len().saturating_add(2));
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
            r"
[commands.slow_cmd]
{blocker}
timeout_secs = 1
"
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
                .stderr(predicate::str::contains(".ops.toml\": TOML parse error"));
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
                .failure()
                .stderr(predicate::str::contains(".ops.toml\": TOML parse error"));
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
#[cfg_attr(
    not(feature = "stack-rust"),
    ignore = "the about header is only populated by the rust stack provider"
)]
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
/// live about-render path (`project_identity` provider + card render). The
/// previous assertion-only-on-`.success()` shape would have passed a
/// regression that silently ignored `--refresh` and emitted nothing; pin
/// the rendered card content via the same stable `project` marker that
/// `cli_about_shows_header` uses, and seed `.ops.toml` with `[about].fields`
/// so the field-filter does not erase the marker.
#[test]
#[cfg_attr(
    not(feature = "stack-rust"),
    ignore = "--refresh only has an observable effect with the rust stack provider"
)]
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

/// TEST-31: `ops about loc` is documented in the README, so it is covered
/// as a spawned command — the subpage only works if arg parsing, the
/// `rust-loc` provider, the `DuckDB` ingest and the renderer all line up,
/// and none of the unit tests exercise that chain end to end.
///
/// The provider walks the workspace, so the fixture carries a `#[cfg(test)]`
/// block: the whole point of the page is that those lines are attributed to
/// Test rather than Production, which is exactly what a line-based counter
/// would get wrong.
#[test]
#[cfg(feature = "duckdb")]
#[cfg_attr(not(feature = "stack-rust"), ignore)]
fn cli_about_loc_splits_production_from_test_lines() {
    let dir = temp_dir();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "demo"
version = "0.1.0"
edition = "2021"
"#,
    )
    .expect("write Cargo.toml");
    std::fs::create_dir_all(dir.path().join("src")).expect("mkdir src");
    std::fs::write(
        dir.path().join("src/lib.rs"),
        r"//! Demo crate.

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[cfg(test)]
mod tests {
    use super::add;

    #[test]
    fn adds() {
        assert_eq!(add(1, 2), 3);
    }
}
",
    )
    .expect("write lib.rs");

    ops()
        .arg("about")
        .arg("loc")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Production"))
        .stdout(predicate::str::contains("Test"))
        .stdout(predicate::str::contains("Docs"))
        .stdout(predicate::str::contains("code lines in"));
}

/// The page is Rust-only: on any other stack the `rust-loc` provider is
/// never registered, and the command must say so and exit 0 rather than
/// failing or printing an empty table.
#[test]
#[cfg(feature = "duckdb")]
fn cli_about_loc_without_rust_reports_no_data() {
    let dir = temp_dir();
    std::fs::write(dir.path().join("package.json"), "{}").expect("write package.json");

    ops()
        .arg("about")
        .arg("loc")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("No Rust LOC data available."));
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
        .stderr(predicate::str::contains("invalid.toml\": TOML parse error"));
}

// -- Pre-commit contract: fixers and checkers (TEST-31 / TASK-1737) --
//
// The README states the contract explicitly: these commands exit non-zero when
// they change (or reject) a file, so a git pre-commit hook fails the commit.
// That mapping lives in `subcommands.rs::run_text_fixer` /
// `run_config_checker` and had no test at any level: a regression that
// inverted the branch would make every `ops tw` / `ops eof` / `ops check-json`
// hook pass silently while files were rewritten under the committer.

/// Run a fixer/checker as a spawned process in `dir` and return
/// (success, stdout).
fn ops_in(dir: &Path, args: &[&str]) -> assert_cmd::assert::Assert {
    ops().args(args).current_dir(dir).assert()
}

#[test]
fn cli_trailing_whitespace_rewrites_and_exits_non_zero() {
    let dir = temp_dir();
    let file = dir.path().join("dirty.txt");
    std::fs::write(&file, "a  \nb\n").expect("write fixture");

    ops_in(dir.path(), &["trailing-whitespace"])
        .failure()
        .stdout(predicate::str::contains("dirty.txt"))
        .stdout(predicate::str::contains("1 changed"));
    assert_eq!(
        std::fs::read_to_string(&file).expect("read back"),
        "a\nb\n",
        "the file must actually be rewritten"
    );

    // Second run: nothing left to fix, so the pre-commit hook passes.
    ops_in(dir.path(), &["trailing-whitespace"])
        .success()
        .stdout(predicate::str::contains("0 changed"));
}

#[test]
fn cli_trailing_whitespace_tw_alias_honours_the_same_contract() {
    let dir = temp_dir();
    std::fs::write(dir.path().join("dirty.txt"), "a  \n").expect("write fixture");
    ops_in(dir.path(), &["tw"])
        .failure()
        .stdout(predicate::str::contains("trailing-whitespace"));
    ops_in(dir.path(), &["tw"]).success();
}

#[test]
fn cli_end_of_file_fixer_rewrites_and_exits_non_zero() {
    let dir = temp_dir();
    let file = dir.path().join("no-newline.txt");
    std::fs::write(&file, "x").expect("write fixture");

    ops_in(dir.path(), &["end-of-file-fixer"])
        .failure()
        .stdout(predicate::str::contains("no-newline.txt"))
        .stdout(predicate::str::contains("1 changed"));
    assert_eq!(
        std::fs::read_to_string(&file).expect("read back"),
        "x\n",
        "the missing trailing newline must actually be added"
    );

    ops_in(dir.path(), &["end-of-file-fixer"])
        .success()
        .stdout(predicate::str::contains("0 changed"));
}

#[test]
fn cli_end_of_file_fixer_eof_alias_honours_the_same_contract() {
    let dir = temp_dir();
    std::fs::write(dir.path().join("no-newline.txt"), "x").expect("write fixture");
    ops_in(dir.path(), &["eof"])
        .failure()
        .stdout(predicate::str::contains("end-of-file-fixer"));
    ops_in(dir.path(), &["eof"]).success();
}

#[test]
fn cli_check_json_fails_on_malformed_and_passes_on_valid() {
    let dir = temp_dir();
    std::fs::write(dir.path().join("bad.json"), "{bad").expect("write fixture");
    ops_in(dir.path(), &["check-json"])
        .failure()
        .stdout(predicate::str::contains("bad.json"))
        .stdout(predicate::str::contains("1 failed"));

    std::fs::remove_file(dir.path().join("bad.json")).expect("remove fixture");
    std::fs::write(dir.path().join("good.json"), "{\"a\": 1}\n").expect("write fixture");
    ops_in(dir.path(), &["check-json"])
        .success()
        .stdout(predicate::str::contains("0 failed"));
}

#[test]
fn cli_check_yaml_fails_on_malformed_and_passes_on_valid() {
    let dir = temp_dir();
    std::fs::write(dir.path().join("bad.yaml"), "a: [1\n").expect("write fixture");
    ops_in(dir.path(), &["check-yaml"])
        .failure()
        .stdout(predicate::str::contains("bad.yaml"))
        .stdout(predicate::str::contains("1 failed"));

    std::fs::remove_file(dir.path().join("bad.yaml")).expect("remove fixture");
    std::fs::write(dir.path().join("good.yaml"), "a: 1\n").expect("write fixture");
    ops_in(dir.path(), &["check-yaml"])
        .success()
        .stdout(predicate::str::contains("0 failed"));
}

// -- `ops sec` (TEST-31 / TASK-1737) --

#[test]
fn cli_sec_dry_run_prints_the_plan_without_requiring_trivy() {
    let dir = temp_dir();
    // An empty PATH makes the "is trivy installed" probe fail deterministically,
    // so the preview is proven not to depend on Trivy being present.
    let empty = temp_dir();
    ops()
        .args(["sec", "--dry-run"])
        .env("PATH", empty.path())
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("ops sec — scan plan:"))
        .stdout(predicate::str::contains("[run ] secrets"))
        .stdout(predicate::str::contains("[skip] vulnerabilities"))
        .stdout(predicate::str::contains("[skip] misconfiguration"))
        // The heads-up keeps the preview honest about a live run.
        .stderr(predicate::str::contains("trivy not found on PATH"));
}

#[test]
fn cli_sec_with_every_scan_skipped_fails_closed() {
    // SEC-31: exiting 0 here would report a clean scan for a gate that never
    // ran. Reaches the empty-selection branch before the Trivy probe, so an
    // empty PATH must not change the outcome.
    let dir = temp_dir();
    let empty = temp_dir();
    ops()
        .args([
            "sec",
            "--skip",
            "secrets",
            "--skip",
            "vuln",
            "--skip",
            "misconfig",
        ])
        .env("PATH", empty.path())
        .current_dir(dir.path())
        .assert()
        .failure()
        .stdout(predicate::str::contains("no scans ran"))
        .stdout(predicate::str::contains("skipped (--skip)"));
}

// -- `ops extension` (TEST-31 / TASK-1737) --

#[test]
fn cli_extension_list_renders_the_extension_table() {
    let dir = temp_dir();
    ops_in(dir.path(), &["extension", "list"])
        .success()
        .stdout(predicate::str::contains("Generic extensions:"))
        .stdout(predicate::str::contains("Name"))
        .stdout(predicate::str::contains("Description"))
        .stdout(predicate::str::contains("text-fixers"));
}

#[test]
fn cli_extension_show_renders_the_object_header() {
    let dir = temp_dir();
    ops_in(dir.path(), &["extension", "show", "text-fixers"])
        .success()
        .stdout(predicate::str::contains("EXTENSION: text-fixers"))
        .stdout(predicate::str::contains("Shortname:"))
        .stdout(predicate::str::contains("Commands:"));
}

#[test]
fn cli_extension_show_unknown_name_fails_and_lists_the_alternatives() {
    let dir = temp_dir();
    ops_in(dir.path(), &["extension", "show", "no-such-extension"])
        .failure()
        .stderr(predicate::str::contains("extension not found"))
        .stderr(predicate::str::contains("Available:"));
}

// -- Interactive commands refuse without a terminal (TEST-31 / TASK-1737) --
//
// A spawned process has piped stdio, so these exercise the real non-TTY gate.

#[test]
fn cli_new_command_without_a_terminal_refuses() {
    let dir = temp_dir();
    ops_in(dir.path(), &["new-command"])
        .failure()
        .stderr(predicate::str::contains("new-command"))
        .stderr(predicate::str::contains("requires an interactive terminal"));
}

#[test]
fn cli_import_makefile_without_a_terminal_refuses() {
    let dir = temp_dir();
    std::fs::write(dir.path().join("Makefile"), "build:\n\techo hi\n").expect("write Makefile");
    ops_in(dir.path(), &["import-makefile"])
        .failure()
        .stderr(predicate::str::contains("import-makefile"))
        .stderr(predicate::str::contains("requires an interactive terminal"));
}

// -- `ops create-review-tasks` and `ops run-before-push` (TEST-31 / TASK-2021) --
//
// TASK-1737 covered the rest of the README command table as spawned processes
// but left these two out of its acceptance criteria. `run-before-push` is the
// gate-shaped one: a git pre-push hook execs it and its exit code decides
// whether the push proceeds, so a regression collapsing a failing command to
// exit 0 would silently disarm the hook.

/// Minimal single-package Cargo manifest — enough for stack detection to
/// resolve Rust and for the `review_targets` provider to answer with one
/// target.
const SOLO_CARGO_TOML: &str = r#"[package]
name = "demo"
version = "0.1.0"
edition = "2021"
"#;

/// `--dry-run` must render the plan the next real run would write, and write
/// nothing: the report is the only observable effect. Rust-only because the
/// `review_targets` provider ships with the rust stack.
#[test]
#[cfg_attr(
    not(feature = "stack-rust"),
    ignore = "review_targets is only registered by the rust stack provider"
)]
fn cli_create_review_tasks_dry_run_reports_the_plan_without_writing() {
    let dir = temp_dir();
    std::fs::write(dir.path().join("Cargo.toml"), SOLO_CARGO_TOML).expect("write Cargo.toml");
    let tasks = dir.path().join(".backlog/tasks");
    std::fs::create_dir_all(&tasks).expect("create .backlog/tasks");

    ops()
        .args(["create-review-tasks", "--dry-run"])
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "would create TASK-0001 review-request-",
        ))
        .stdout(predicate::str::contains("(1 subtasks)"))
        .stdout(predicate::str::contains(
            "would create TASK-0001.01 REVIEW: Run skill code-review-rust against demo",
        ))
        .stdout(predicate::str::contains(
            "list subtasks: backlog task list --parent TASK-0001",
        ));

    let written: Vec<_> = std::fs::read_dir(&tasks)
        .expect("read .backlog/tasks")
        .map(|e| e.expect("dir entry").file_name())
        .collect();
    assert!(
        written.is_empty(),
        "a dry run must not create task files, found: {written:?}"
    );
}

/// The write form is the one that has an effect on disk, and the report is
/// only printed once the whole task set exists — so "created" on stdout and
/// the files under `.backlog/tasks` have to agree. The fixture's backlog tree
/// is a tempdir, never the developer's.
#[test]
#[cfg_attr(
    not(feature = "stack-rust"),
    ignore = "review_targets is only registered by the rust stack provider"
)]
fn cli_create_review_tasks_writes_the_task_set() {
    let dir = temp_dir();
    std::fs::write(dir.path().join("Cargo.toml"), SOLO_CARGO_TOML).expect("write Cargo.toml");
    let tasks = dir.path().join(".backlog/tasks");
    std::fs::create_dir_all(&tasks).expect("create .backlog/tasks");

    ops()
        .arg("create-review-tasks")
        .current_dir(dir.path())
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "created TASK-0001 review-request-",
        ))
        .stdout(predicate::str::contains(
            "created TASK-0001.01 REVIEW: Run skill code-review-rust against demo",
        ));

    let mut written: Vec<String> = std::fs::read_dir(&tasks)
        .expect("read .backlog/tasks")
        .map(|e| e.expect("dir entry").file_name().to_string_lossy().into())
        .collect();
    written.sort();
    assert_eq!(
        written.len(),
        2,
        "one main task plus one subtask must exist, found: {written:?}"
    );
    assert!(
        written[0].starts_with("task-0001 - review-request-"),
        "main task file must be named after the main task, found: {written:?}"
    );
    assert!(
        written[1].starts_with("task-0001.01 - REVIEW-"),
        "subtask file must be named after the subtask, found: {written:?}"
    );
}

/// Without a `.backlog/tasks` tree there is nothing to allocate ids against.
/// The refusal has to name the missing directory, or the operator's only
/// signal is a non-zero exit.
#[test]
fn cli_create_review_tasks_without_a_backlog_tree_fails() {
    let dir = temp_dir();
    std::fs::write(dir.path().join("Cargo.toml"), SOLO_CARGO_TOML).expect("write Cargo.toml");

    ops()
        .args(["create-review-tasks", "--dry-run"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(".backlog/tasks"));
}

/// `ops` clears every `OPS_*` variable already; `SKIP_OPS_RUN_BEFORE_PUSH`
/// does not match that prefix, so an ambient bypass in the developer's shell
/// would turn both exit-code assertions below into vacuous successes.
fn ops_hook() -> Command {
    let mut cmd = ops();
    cmd.env_remove("SKIP_OPS_RUN_BEFORE_PUSH");
    cmd
}

#[test]
fn cli_run_before_push_propagates_the_configured_command_exit_code() {
    for (program, expect_success) in [("true", true), ("false", false)] {
        let dir = temp_dir();
        write_ops_toml(
            dir.path(),
            &format!(
                r#"
[commands.run-before-push]
program = "{program}"
"#
            ),
        );

        let assertion = ops_hook()
            .arg("run-before-push")
            .current_dir(dir.path())
            .assert();
        if expect_success {
            assertion.success();
        } else {
            assertion.failure();
        }
    }
}

/// A hook whose command is not configured must not exit 0: git reads success
/// as "the hook passed" and would let the push through unchecked. Without a
/// terminal there is nothing to prompt on, so the run bails immediately.
#[test]
fn cli_run_before_push_without_a_configured_command_fails_closed() {
    let dir = temp_dir();
    write_ops_toml(
        dir.path(),
        r#"[output]
theme = "classic"
"#,
    );

    ops_hook()
        .arg("run-before-push")
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "no 'run-before-push' command configured",
        ));
}

/// `install` picks the hook's commands from an `inquire` multi-select, so a
/// spawned process (piped stdio) must refuse rather than render a picker
/// nobody can answer. The `.git` directory is required first: the install
/// path resolves the repository before it reaches the TTY gate.
#[test]
fn cli_run_before_push_install_without_a_terminal_refuses() {
    let dir = temp_dir();
    let git_dir = dir.path().join(".git");
    std::fs::create_dir(&git_dir).expect("create .git");
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    ops_hook()
        .args(["run-before-push", "install"])
        .current_dir(dir.path())
        .assert()
        .failure()
        .stderr(predicate::str::contains("run-before-push install"))
        .stderr(predicate::str::contains("requires an interactive terminal"));
}

// -- `ops deps` (TEST-31 / TASK-2030) --
//
// TASK-1845 pinned `run_deps` / `ensure_tools` / `DepsProvider::provide`
// in-process. Three things live only above that seam and were asserted
// nowhere: the exit code `main` derives from the `dependency issues found`
// error, the split of the report (stdout) from that error (stderr), and the
// `--refresh` flag reaching `DepsOptions`.
//
// `ops deps` shells out to `cargo upgrade` and `cargo deny`. `run_cargo`
// resolves the binary through `$CARGO`, so a shim on that variable stands in
// for both cargo-edit and cargo-deny and keeps the test off the network. The
// shim is `/bin/sh`, hence unix-only.

/// A stand-in `cargo` that answers the two `--version` probes `ensure_tools`
/// makes and the two collection commands `DepsProvider` spawns. `cargo
/// upgrade --dry-run` prints nothing, which the upgrade parser reads as "no
/// upgrades available"; `cargo deny` replays `deny_diagnostics` on stderr and
/// exits `deny_exit`, cargo-deny's own contract for "issues found".
#[cfg(all(unix, feature = "stack-rust"))]
fn write_fake_cargo(dir: &Path, deny_exit: u8, deny_diagnostics: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;

    let path = dir.join("fake-cargo");
    // The heredoc keeps the JSON diagnostics free of shell quoting; its
    // delimiter is namespaced so no diagnostic line can close it early.
    std::fs::write(
        &path,
        format!(
            r#"#!/bin/sh
case "$1 $2" in
  "upgrade --version") echo "cargo-upgrade 0.0.0-fake"; exit 0 ;;
  "deny --version") echo "cargo-deny 0.0.0-fake"; exit 0 ;;
  "upgrade --dry-run") exit 0 ;;
esac
if [ "$1" = "deny" ]; then
  cat >&2 <<'OPS_FAKE_DENY_EOF'
{deny_diagnostics}
OPS_FAKE_DENY_EOF
  exit {deny_exit}
fi
echo "fake cargo: unexpected invocation: $*" >&2
exit 127
"#
        ),
    )
    .expect("write fake cargo");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    path
}

/// One cargo-deny `error` advisory, in the newline-delimited JSON shape
/// `--format json` emits on stderr.
#[cfg(all(unix, feature = "stack-rust"))]
const DENY_ADVISORY_DIAGNOSTIC: &str = r#"{"type":"diagnostic","fields":{"severity":"error","code":"vulnerability","message":"fixture advisory","advisory":{"id":"RUSTSEC-0000-0000","package":"demo","title":"fixture advisory"}}}"#;

/// A fixture project plus the `ops deps` invocation that runs against it,
/// with `$CARGO` pointed at the shim.
#[cfg(all(unix, feature = "stack-rust"))]
fn deps_cmd(dir: &Path, deny_exit: u8, deny_diagnostics: &str) -> Command {
    std::fs::write(dir.join("Cargo.toml"), SOLO_CARGO_TOML).expect("write Cargo.toml");
    let cargo = write_fake_cargo(dir, deny_exit, deny_diagnostics);
    let mut cmd = ops();
    cmd.env("CARGO", cargo).arg("deps").current_dir(dir);
    cmd
}

/// A clean report is the "scores green" case: exit 0, and the rendered
/// report — not a bare exit status — is what a pipeline reads off stdout.
#[test]
#[cfg(all(unix, feature = "stack-rust"))]
fn cli_deps_exits_zero_and_renders_the_report_on_stdout_when_clean() {
    let dir = temp_dir();
    deps_cmd(dir.path(), 0, "")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dependency Health Report"))
        .stdout(predicate::str::contains("Advisories"))
        .stdout(predicate::str::contains("None"));
}

/// The product's contract: `ops deps` fails CI when the report carries
/// actionable issues. The exit code is `main`'s translation of `run_deps`'
/// `dependency issues found` error and was pinned nowhere above the library.
///
/// The stream split is asserted in the same run because it is the other half
/// of what a consumer sees: a change routing the report to stderr (or the
/// failure to stdout) would break every pipeline reading `ops deps` while
/// leaving the whole suite green.
#[test]
#[cfg(all(unix, feature = "stack-rust"))]
fn cli_deps_fails_and_splits_report_from_error_when_issues_are_found() {
    let dir = temp_dir();
    deps_cmd(dir.path(), 1, DENY_ADVISORY_DIAGNOSTIC)
        .assert()
        .failure()
        .stdout(predicate::str::contains("Dependency Health Report"))
        .stdout(predicate::str::contains("Advisories"))
        .stdout(predicate::str::contains("1 error"))
        .stdout(predicate::str::contains("dependency issues found").not())
        .stderr(predicate::str::contains("dependency issues found"));
}

/// `--refresh` is plumbed `CoreSubcommand::Deps { refresh }` → `run_deps` →
/// `DepsOptions::new(refresh)` → `Context::with_refresh`. The context's cache
/// is per-process and `ops deps` queries the provider once, so refreshing has
/// no *observable* effect on a single run — what a spawned test can pin is
/// that the flag is accepted at this position and the deps path still runs to
/// a rendered report. `deps_refresh_flag_reaches_deps_options` in
/// `subcommands.rs` pins the value carried across the seam.
#[test]
#[cfg(all(unix, feature = "stack-rust"))]
fn cli_deps_accepts_refresh_and_still_renders_the_report() {
    let dir = temp_dir();
    deps_cmd(dir.path(), 0, "")
        .arg("--refresh")
        .assert()
        .success()
        .stdout(predicate::str::contains("Dependency Health Report"));
}
