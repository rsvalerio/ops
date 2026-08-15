//! `cargo --list` parsing and cargo-tool installation detection.

use ops_core::subprocess::resolve_cargo_bin;
use std::collections::HashSet;
use std::process::Command;

use super::path::check_binary_installed;
use super::timeout::{run_probe_capturing, ProbeOutcome};

/// PATTERN-1 / TASK-1101: `cargo --list` enumerates *every* subcommand cargo
/// knows about — built-ins ship inside the cargo binary itself, not as
/// separately installable `cargo-*` crates. Filtering them out forces a
/// `tools.toml` `Cargo`-source spec to fall through to the PATH probe.
///
/// PATTERN-1 / TASK-1582 — drift strategy. Cargo's built-in surface evolves
/// (e.g. `info` was added in 1.74, future built-ins may follow), so a
/// hand-curated list silently goes stale: a new built-in starts appearing
/// in `cargo --list` and is misread as an installed cargo tool. We keep
/// the list (the alternatives — parsing `cargo --list -v` paths or
/// distinguishing via the cargo binary's directory — are not portable
/// across cargo versions and rustup proxies) but pair it with the
/// [`cargo_builtins_list_is_in_sync`] regression test in this module:
/// for every entry `cargo --list` produces that we do not already
/// recognise as a built-in or alias, the test asserts that a
/// `cargo-<name>` binary exists on `$PATH`. If neither is true, cargo
/// has gained a new built-in we do not track and CI fails — surfacing
/// the drift before it ships.
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
    "git-checkout",
    "help",
    "info",
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

/// PATTERN-1 / TASK-1582: cargo also ships built-in aliases that surface
/// in `cargo --list` as e.g. `b    alias: build`. They have no
/// `cargo-<name>` binary; the drift regression test must accept them.
#[cfg(test)]
const CARGO_BUILTIN_ALIASES: &[&str] = &["b", "c", "d", "r", "rm", "t"];

/// API / TASK-1200: returns [`ProbeOutcome::Failed`] when `cargo --list`
/// itself cannot be answered (timeout / IO / non-zero exit). The PATH
/// fallback runs only when `cargo --list` answered.
/// PERF/ERR (TASK-1665): `--color never` is not cosmetic. `cargo --list`
/// colourises its output whenever colour is forced — `CARGO_TERM_COLOR=always`
/// in the environment, which this repo's own CI sets — and then every entry
/// arrives wrapped in ANSI escapes (`\x1b[1m\x1b[96madd`). The
/// whitespace-splitting parser below reads that as the subcommand name, so no
/// entry ever matches and every cargo tool is reported as not installed.
/// Passing the flag explicitly makes the probe independent of the caller's
/// environment rather than relying on cargo's TTY auto-detection.
#[must_use]
pub fn check_cargo_tool_installed(name: &str) -> ProbeOutcome<bool> {
    let mut cmd = Command::new(resolve_cargo_bin());
    cmd.args(["--color", "never", "--list"]);
    match run_probe_capturing(&mut cmd, "cargo --list") {
        ProbeOutcome::Ok(stdout) => {
            ProbeOutcome::Ok(is_in_cargo_list(&stdout, name) || check_binary_installed(name))
        }
        ProbeOutcome::Failed => ProbeOutcome::Failed,
    }
}

/// Strip ANSI CSI sequences (`ESC [ … final-byte`) from a token.
///
/// TASK-1665: defence in depth beside the `--color never` flag on the probe
/// invocation. This function is reachable with arbitrary stdout — notably via
/// the public `capture_cargo_list` — so it should not depend on the caller
/// having suppressed colour. Borrows unchanged when there is no escape to
/// remove, which is the normal path.
fn strip_ansi(token: &str) -> std::borrow::Cow<'_, str> {
    if !token.contains('\x1b') {
        return std::borrow::Cow::Borrowed(token);
    }
    let mut out = String::with_capacity(token.len());
    let mut chars = token.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\x1b' {
            out.push(c);
            continue;
        }
        // Only CSI (`ESC [`) is consumed; a lone ESC is dropped and the
        // following character is preserved.
        if chars.peek() == Some(&'[') {
            chars.next();
            // Parameter/intermediate bytes run until a final byte in @..~.
            for p in chars.by_ref() {
                if ('@'..='~').contains(&p) {
                    break;
                }
            }
        }
    }
    std::borrow::Cow::Owned(out)
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
            .is_some_and(|cmd| strip_ansi(cmd) == cargo_name)
    })
}

/// Capture the raw stdout of `cargo --list` once.
#[must_use]
pub fn capture_cargo_list() -> ProbeOutcome<String> {
    let mut cmd = Command::new(resolve_cargo_bin());
    // TASK-1665: see `check_cargo_tool_installed` — colour must be off or the
    // parser reads ANSI escapes as part of the subcommand name.
    cmd.args(["--color", "never", "--list"]);
    run_probe_capturing(&mut cmd, "cargo --list")
}

/// PERF-3 / TASK-1616: pre-tokenise `cargo --list` stdout into a hash set
/// of installed subcommand names so [`is_in_cargo_set`] is O(1) per tool
/// instead of an O(L) line scan for each of T tools (the prior shape was
/// O(T × L) per `collect_tools` sweep). Built-in subcommands are filtered
/// out at index-build time, so a lookup for a built-in name returns
/// `false` regardless of what's in the set.
pub(crate) fn cargo_list_index(stdout: &str) -> HashSet<String> {
    stdout
        .lines()
        .filter_map(|line| line.split_whitespace().next())
        .filter(|word| !CARGO_BUILTIN_SUBCOMMANDS.contains(word))
        .map(str::to_string)
        .collect()
}

/// PERF-3 / TASK-1616: O(1) variant of [`is_in_cargo_list`] over a
/// precomputed [`cargo_list_index`] set.
pub(crate) fn is_in_cargo_set(set: &HashSet<String>, name: &str) -> bool {
    let cargo_name = name.strip_prefix("cargo-").unwrap_or(name);
    if cargo_name.is_empty() {
        return false;
    }
    set.contains(cargo_name)
}

#[cfg(test)]
mod cargo_builtins_drift_tests {
    use super::*;
    use crate::probe::path::check_binary_installed;

    /// TASK-1665: the drift test above shells out to `cargo --list`, and the
    /// probe must be immune to colour being forced on. With
    /// `CARGO_TERM_COLOR=always` set — as this repo's CI does — an unguarded
    /// `cargo --list` returns entries wrapped in ANSI escapes, the
    /// whitespace-splitting parser reads `\x1b[1m\x1b[96madd` as the
    /// subcommand name, nothing matches, and every cargo tool is reported as
    /// not installed. This was a live production bug: any user with that
    /// variable exported got wrong answers from `ops`. It surfaced only once
    /// CI started running the full suite (TASK-1664).
    ///
    /// Asserts on the parser rather than the subprocess so it holds regardless
    /// of which cargo is installed on the machine running the tests.
    #[test]
    fn is_in_cargo_list_is_not_fooled_by_ansi_colour_codes() {
        // Exactly the shape `cargo --list` emits under CARGO_TERM_COLOR=always.
        let coloured = "Installed Commands:\n    \x1b[1m\x1b[96mnextest\x1b[0m Run tests\n";
        let plain = "Installed Commands:\n    nextest Run tests\n";

        assert!(
            is_in_cargo_list(plain, "nextest"),
            "sanity: the plain form must match"
        );
        assert!(
            is_in_cargo_list(coloured, "nextest"),
            "colourised `cargo --list` output must still match; the probe passes \
             `--color never`, but if that guard is dropped this parser silently \
             reports every cargo tool as missing"
        );
    }

    /// PATTERN-1 / TASK-1582: every non-alias entry in `cargo --list` must
    /// either be tracked as a built-in or correspond to an installed
    /// `cargo-<name>` binary. A new entry that satisfies neither indicates
    /// cargo has grown a new built-in that [`CARGO_BUILTIN_SUBCOMMANDS`]
    /// does not yet track — the test fails so the drift surfaces here
    /// instead of in user-facing probes (mis-reporting an installed cargo
    /// tool that happens to share a name with the new built-in).
    #[test]
    fn cargo_builtins_list_is_in_sync() {
        let ProbeOutcome::Ok(stdout) = capture_cargo_list() else {
            // Probe failed (cargo missing/timeout/non-zero). The drift
            // gate is a best-effort regression check; a flaky cargo
            // invocation should not fail the unit-test suite.
            eprintln!("cargo_builtins_list_is_in_sync: `cargo --list` did not answer; skipping");
            return;
        };

        let mut unknown: Vec<String> = Vec::new();
        for raw in stdout.lines() {
            let line = raw.trim();
            if line.is_empty() || line.ends_with(':') {
                continue;
            }
            let Some(name) = line.split_whitespace().next() else {
                continue;
            };
            if CARGO_BUILTIN_ALIASES.contains(&name) {
                continue;
            }
            if CARGO_BUILTIN_SUBCOMMANDS.contains(&name) {
                continue;
            }
            // Externally installed cargo subcommands ship a `cargo-<name>`
            // binary on PATH. If a `cargo --list` entry has neither a
            // builtin/alias entry here nor a corresponding binary, the
            // most likely explanation is a new built-in we missed.
            let cargo_bin = format!("cargo-{name}");
            if check_binary_installed(&cargo_bin) {
                continue;
            }
            unknown.push(name.to_string());
        }

        assert!(
            unknown.is_empty(),
            "cargo --list entries are neither tracked as built-ins/aliases nor backed by a \
             cargo-<name> binary on PATH; this likely indicates cargo gained new built-ins. \
             Add them to CARGO_BUILTIN_SUBCOMMANDS (or CARGO_BUILTIN_ALIASES) in \
             src/probe/cargo.rs: {unknown:?}"
        );
    }
}
