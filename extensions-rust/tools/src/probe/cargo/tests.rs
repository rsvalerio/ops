//! Tests for `cargo --list` membership and cargo-tool detection.

use super::*;
use crate::probe::ProbeOutcome;

#[test]
fn cargo_list_finds_subcommand() {
    let stdout = "    bench\n    build\n    nextest\n    check\n";
    assert!(is_in_cargo_list(stdout, "cargo-nextest"));
}

#[test]
fn cargo_list_strips_cargo_prefix() {
    let stdout = "    nextest\n";
    assert!(is_in_cargo_list(stdout, "cargo-nextest"));
}

#[test]
fn cargo_list_no_prefix_match() {
    let stdout = "    watch\n";
    assert!(is_in_cargo_list(stdout, "watch"));
}

#[test]
fn cargo_list_not_found() {
    let stdout = "    bench\n    build\n    check\n";
    assert!(!is_in_cargo_list(stdout, "cargo-nextest"));
}

#[test]
fn cargo_list_empty() {
    assert!(!is_in_cargo_list("", "cargo-nextest"));
}

#[test]
fn cargo_list_ignores_description_suffix() {
    let stdout = "    nextest              Next-gen test runner\n";
    assert!(is_in_cargo_list(stdout, "cargo-nextest"));
}

#[test]
fn cargo_list_partial_name_no_match() {
    let stdout = "    nextest\n";
    assert!(!is_in_cargo_list(stdout, "cargo-nextestx"));
}

#[test]
fn cargo_list_similar_prefix_no_match() {
    let stdout = "    nextest\n";
    assert!(!is_in_cargo_list(stdout, "cargo-next"));
}

/// TASK-0526: an empty-after-strip name (literal "cargo-" or "") must not
/// match a line that begins with whitespace. The empty token from
/// split_whitespace was previously equal to the empty stripped name, so any
/// indented line was reported as installed.
#[test]
fn cargo_list_empty_name_after_strip_is_rejected() {
    let stdout = "    nextest\n    watch\n";
    assert!(!is_in_cargo_list(stdout, "cargo-"));
    assert!(!is_in_cargo_list(stdout, ""));
}

/// PATTERN-1 / TASK-1101: built-in cargo subcommands (e.g. `build`, `check`,
/// `test`, `run`) appear in `cargo --list` because they're shipped inside the
/// cargo binary itself — not because anyone ran `cargo install cargo-build`.
/// A `tools.toml` entry that collides with a built-in name must not be
/// reported as installed via the membership check; it should fall through to
/// the PATH probe so a real `cargo-<name>` executable still resolves.
#[test]
fn cargo_list_rejects_builtin_subcommand_named_build() {
    let stdout = "    build\n    cargo-foo\n";
    assert!(!is_in_cargo_list(stdout, "build"));
    assert!(!is_in_cargo_list(stdout, "cargo-build"));
}

#[test]
fn cargo_list_rejects_other_common_builtins() {
    let stdout = "    check\n    test\n    run\n    clippy\n    fmt\n    update\n";
    for builtin in ["check", "test", "run", "clippy", "fmt", "update"] {
        assert!(
            !is_in_cargo_list(stdout, builtin),
            "built-in {builtin} must not match"
        );
    }
}

#[test]
fn cargo_list_still_resolves_real_third_party_among_builtins() {
    // Mixed listing: real `cargo install`-ed tools alongside built-ins.
    // `cargo-watch` / `cargo-nextest`-style entries must still resolve.
    let stdout = "    bench\n    build\n    check\n    watch\n    nextest\n";
    assert!(is_in_cargo_list(stdout, "cargo-watch"));
    assert!(is_in_cargo_list(stdout, "watch"));
    assert!(is_in_cargo_list(stdout, "cargo-nextest"));
}

#[test]
#[ignore = "requires rustup + cargo-fmt installed; run with: cargo test -- --ignored"]
fn check_cargo_tool_installed_fmt() {
    // cargo-fmt ships with rustup, should always be present
    assert!(matches!(
        check_cargo_tool_installed("cargo-fmt"),
        ProbeOutcome::Ok(true)
    ));
}

#[test]
#[serial_test::serial]
fn check_cargo_tool_installed_nonexistent() {
    assert!(matches!(
        check_cargo_tool_installed("cargo-nonexistent-abc123"),
        ProbeOutcome::Ok(false) | ProbeOutcome::Failed
    ));
}

/// PORT (TASK-0792): probe spawns must honour `$CARGO` so they invoke the
/// same toolchain binary the parent cargo selected, mirroring
/// `ops_core::subprocess::run_cargo`. The fake script below prints a
/// distinctive subcommand line; if the probe ever falls back to the real
/// `cargo` on `$PATH`, the assertion will fail.
#[cfg(unix)]
#[test]
#[serial_test::serial]
fn check_cargo_tool_installed_honours_cargo_env() {
    use crate::probe::check_cargo_tool_installed;
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().expect("tempdir");
    let fake = dir.path().join("cargo");
    // TASK-1665: scan every argument rather than matching `$1`. The probe now
    // passes `--color never --list`, so `$1` is `--color` and a positional
    // check silently stops matching — which is exactly how this fixture broke
    // when that flag was added. Position-independent, so a future argument
    // change cannot quietly turn this test into a no-op.
    std::fs::write(
        &fake,
        "#!/bin/sh\nfor a in \"$@\"; do\n  if [ \"$a\" = \"--list\" ]; then \
         echo '    fake-marker-tool   A fake'; fi\ndone\n",
    )
    .unwrap();
    let mut perms = std::fs::metadata(&fake).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(&fake, perms).unwrap();

    // SAFETY: serial_test::serial guards against concurrent env mutation; no
    // background thread reads CARGO during the test body.
    unsafe { std::env::set_var("CARGO", &fake) };
    let installed = check_cargo_tool_installed("fake-marker-tool");
    unsafe { std::env::remove_var("CARGO") };

    assert!(
        matches!(installed, ProbeOutcome::Ok(true)),
        "probe must invoke the binary at $CARGO; falling back to PATH would not list `fake-marker-tool`"
    );
}
