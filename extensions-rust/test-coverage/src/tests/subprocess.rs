//! Cargo argv guards, exit-status checking, and stderr diagnostics.

use crate::parse::format_stderr_diagnostic;
use crate::subprocess::{check_llvm_cov_output, LLVM_COV_ARGS};

/// TASK-1595: the extracted helper must return `Some` when stderr contains
/// bytes and `None` when empty, so the success-path log line fires only
/// when there is actually something to report.
#[test]
fn success_stderr_diagnostic_returns_some_for_nonempty_stderr() {
    let diag = format_stderr_diagnostic(b"warning: something");
    assert!(diag.is_some());
    assert!(
        diag.unwrap().contains("warning"),
        "diagnostic should contain stderr tail"
    );
}

#[test]
fn success_stderr_diagnostic_returns_none_for_empty_stderr() {
    assert!(format_stderr_diagnostic(b"").is_none());
}

#[cfg(unix)]
#[test]
fn check_llvm_cov_output_success() {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(0),
        stdout: b"some output".to_vec(),
        stderr: Vec::new(),
    };
    assert!(check_llvm_cov_output(&output).is_ok());
}

#[cfg(unix)]
#[test]
fn check_llvm_cov_output_failure_includes_stderr_tail() {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(256), // exit code 1
        stdout: Vec::new(),
        stderr: b"error: could not compile\ndetails here\nmore info".to_vec(),
    };
    let err = check_llvm_cov_output(&output).unwrap_err();
    let msg = err.to_string();
    // PATTERN-1 / TASK-1099: format is "cargo llvm-cov exited with status {code}: ...".
    assert!(msg.contains("cargo llvm-cov"), "got: {msg}");
    assert!(msg.contains("status 1"), "exit code must appear: {msg}");
    assert!(
        msg.contains("could not compile"),
        "stderr tail should appear: {msg}"
    );
}

#[cfg(unix)]
#[test]
fn check_llvm_cov_output_failure_empty_stderr() {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(256),
        stdout: Vec::new(),
        stderr: Vec::new(),
    };
    let err = check_llvm_cov_output(&output).unwrap_err();
    // PATTERN-1 / TASK-1099: error message format is now
    // "cargo llvm-cov exited with status {code}: ...".
    assert!(err.to_string().contains("cargo llvm-cov"));
}

/// When cargo-llvm-cov is not installed, cargo exits 101 with a
/// "no such command" error for `llvm-cov` on stderr. The hard-fail error
/// must append the actionable install commands (cargo's own hint points at
/// `cargo search`, which names neither the real crate nor the
/// `llvm-tools-preview` component).
#[cfg(unix)]
#[test]
fn check_llvm_cov_output_missing_subcommand_appends_install_hint() {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(101 << 8),
        stdout: Vec::new(),
        stderr: b"error: no such command: `llvm-cov`\n\nhelp: view all installed \
                  commands with `cargo --list`\nhelp: find a package to install \
                  `llvm-cov` with `cargo search cargo-llvm-cov`"
            .to_vec(),
    };
    let err = check_llvm_cov_output(&output).unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("cargo install cargo-llvm-cov"),
        "install command must appear: {msg}"
    );
    assert!(
        msg.contains("rustup component add llvm-tools-preview"),
        "toolchain component must appear: {msg}"
    );
    // PATTERN-1 / TASK-1099 shape survives ahead of the hint.
    assert!(msg.starts_with("cargo llvm-cov exited with status 101"));
}

/// Ordinary cargo failures (compile errors, test failures) must not carry
/// the missing-tool hint — it would misdirect operators.
#[cfg(unix)]
#[test]
fn check_llvm_cov_output_ordinary_failure_has_no_install_hint() {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(101 << 8),
        stdout: Vec::new(),
        stderr: b"error: could not compile `ops`".to_vec(),
    };
    let msg = check_llvm_cov_output(&output).unwrap_err().to_string();
    assert!(
        !msg.contains("cargo install cargo-llvm-cov"),
        "hint must not appear for non-missing-tool failures: {msg}"
    );
}

/// PATTERN-1 / TASK-1099: non-zero exit codes must appear in the error
/// string so exit 1 (issues), exit 101 (panic), and SIGKILL/None are
/// distinguishable in operator logs.
#[cfg(unix)]
#[test]
fn check_llvm_cov_output_failure_includes_exit_code() {
    use std::os::unix::process::ExitStatusExt;
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(101 << 8),
        stdout: Vec::new(),
        stderr: b"thread 'main' panicked".to_vec(),
    };
    let err = check_llvm_cov_output(&output).expect_err("non-zero must fail");
    let msg = err.to_string();
    assert!(msg.contains("status 101"), "exit code must appear: {msg}");
    assert!(msg.contains("panicked"), "stderr tail must remain: {msg}");
}

/// PATTERN-1 / TASK-1099: a None exit (signal kill, e.g. OOM) is named
/// as `signal` so it's distinguishable from a real cargo failure.
#[cfg(unix)]
#[test]
fn check_llvm_cov_output_failure_signal_kill_says_signal() {
    use std::os::unix::process::ExitStatusExt;
    // signal 9 (SIGKILL) → exit_code() returns None
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(9),
        stdout: Vec::new(),
        stderr: Vec::new(),
    };
    let err = check_llvm_cov_output(&output).expect_err("signal must fail");
    let msg = err.to_string();
    assert!(
        msg.contains("signal") || msg.contains("None"),
        "signal-kill case must be named in error: {msg}"
    );
}

/// ERR-1 / TASK-1057: `cargo llvm-cov` must run with `--no-fail-fast`
/// so a single failing test does not erase the entire coverage signal
/// for the run. TEST-23 / TASK-1554: the regression guard reads the
/// argv slice directly rather than grepping the source text via
/// `include_str!`, so rustfmt re-wraps, helper-function moves, and
/// const-renames cannot make this test rot silently.
#[test]
fn run_cargo_llvm_cov_arg_list_includes_no_fail_fast() {
    assert!(
        LLVM_COV_ARGS.contains(&"--no-fail-fast"),
        "argv must include --no-fail-fast (TASK-1057); current argv: {LLVM_COV_ARGS:?}"
    );
    assert_eq!(
        LLVM_COV_ARGS.last().copied(),
        Some("--json"),
        "TASK-1057: --json must be the final flag (downstream parsing depends on JSON mode); \
         current argv: {LLVM_COV_ARGS:?}"
    );
    // The first flag is the subcommand name; bind it so reordering doesn't
    // silently make this a `cargo nextest --json` invocation.
    assert_eq!(LLVM_COV_ARGS.first().copied(), Some("llvm-cov"));
}

/// The JSON report must go to a file via `--output-path`, not stdout: the
/// report grows with the workspace and an ~8 MB document blows past the
/// `OPS_OUTPUT_BYTE_CAP` stdout cap, silently truncating it into
/// unparseable JSON and erasing the entire coverage signal.
#[test]
fn llvm_cov_argv_appends_output_path_after_static_args() {
    let argv = crate::subprocess::llvm_cov_argv("/tmp/report.json");
    assert_eq!(&argv[..LLVM_COV_ARGS.len()], LLVM_COV_ARGS);
    assert_eq!(
        &argv[LLVM_COV_ARGS.len()..],
        &["--output-path", "/tmp/report.json"],
        "report must be written to a file, not stdout (output-cap truncation); argv: {argv:?}"
    );
}
