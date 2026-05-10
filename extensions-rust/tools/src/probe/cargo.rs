//! `cargo --list` parsing and cargo-tool installation detection.

use ops_core::output::format_error_tail;
use ops_core::subprocess::resolve_cargo_bin;
use std::process::Command;

use super::path::check_binary_installed;
use super::timeout::{run_probe_with_timeout, ProbeOutcome};

/// PATTERN-1 / TASK-1101: `cargo --list` enumerates *every* subcommand cargo
/// knows about — built-ins ship inside the cargo binary itself, not as
/// separately installable `cargo-*` crates. Filtering them out forces a
/// `tools.toml` `Cargo`-source spec to fall through to the PATH probe.
const CARGO_BUILTIN_SUBCOMMANDS: &[&str] = &[
    "add",
    "bench",
    "build",
    "check",
    "clean",
    "clippy",
    "config",
    "doc",
    "fetch",
    "fix",
    "fmt",
    "generate-lockfile",
    "help",
    "init",
    "install",
    "locate-project",
    "login",
    "logout",
    "metadata",
    "new",
    "owner",
    "package",
    "pkgid",
    "publish",
    "read-manifest",
    "remove",
    "report",
    "run",
    "rustc",
    "rustdoc",
    "search",
    "test",
    "tree",
    "uninstall",
    "update",
    "vendor",
    "verify-project",
    "version",
    "yank",
];

/// API / TASK-1200: returns [`ProbeOutcome::Failed`] when `cargo --list`
/// itself cannot be answered (timeout / IO / non-zero exit). The PATH
/// fallback runs only when `cargo --list` answered.
pub fn check_cargo_tool_installed(name: &str) -> ProbeOutcome<bool> {
    let mut cmd = Command::new(resolve_cargo_bin());
    cmd.args(["--list"]);
    let output = match run_probe_with_timeout(&mut cmd, "cargo --list") {
        ProbeOutcome::Ok(o) => o,
        ProbeOutcome::Failed => return ProbeOutcome::Failed,
    };

    if !output.status.success() {
        let stderr_snippet = format_error_tail(&output.stderr, 10);
        tracing::warn!(
            tool = name,
            code = ?output.status.code(),
            stderr = ?stderr_snippet,
            "cargo --list exited non-zero; reporting tool as ProbeFailed"
        );
        return ProbeOutcome::Failed;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    ProbeOutcome::Ok(is_in_cargo_list(&stdout, name) || check_binary_installed(name))
}

pub(crate) fn is_in_cargo_list(stdout: &str, name: &str) -> bool {
    let cargo_name = name.strip_prefix("cargo-").unwrap_or(name);
    if cargo_name.is_empty() {
        return false;
    }
    if CARGO_BUILTIN_SUBCOMMANDS.contains(&cargo_name) {
        return false;
    }
    stdout.lines().any(|line| {
        line.split_whitespace()
            .next()
            .is_some_and(|cmd| cmd == cargo_name)
    })
}

/// Capture the raw stdout of `cargo --list` once.
pub fn capture_cargo_list() -> ProbeOutcome<String> {
    let mut cmd = Command::new(resolve_cargo_bin());
    cmd.args(["--list"]);
    let output = match run_probe_with_timeout(&mut cmd, "cargo --list") {
        ProbeOutcome::Ok(o) => o,
        ProbeOutcome::Failed => return ProbeOutcome::Failed,
    };
    if !output.status.success() {
        return ProbeOutcome::Failed;
    }
    ProbeOutcome::Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}
