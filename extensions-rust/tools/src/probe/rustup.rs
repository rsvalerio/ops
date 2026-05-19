//! `rustup` toolchain / component probes.

use ops_core::subprocess::resolve_rustup_bin;
use std::process::Command;

use super::timeout::{run_probe_capturing, run_probe_with_timeout, ProbeOutcome};

pub fn get_active_toolchain() -> Option<String> {
    // `--quiet` is rustup's global flag, not a subcommand option, so it
    // appears before `show`. ASYNC-6 / TASK-0914: capped at PROBE_TIMEOUT.
    let mut cmd = Command::new(resolve_rustup_bin());
    cmd.args(["--quiet", "show", "active-toolchain"]);
    let output = match run_probe_with_timeout(&mut cmd, "rustup show active-toolchain") {
        ProbeOutcome::Ok(o) => o,
        ProbeOutcome::Failed => return None,
    };

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_active_toolchain(&stdout)
}

/// Parse the toolchain name out of `rustup show active-toolchain` stdout.
///
/// PATTERN-1 / TASK-1078: only the rustup diagnostic prefixes
/// (`error:`, `warning:`, `info:`, `note:`) cause rejection. Match the
/// prefix on the full first whitespace-bounded segment, not as a substring.
///
/// ERR-1 / TASK-1197: rustup commonly emits a leading `info:` progress line
/// before the real toolchain identifier (e.g. `info: syncing channel
/// updates ...\nstable-aarch64-apple-darwin\n`). Skip diagnostic-prefixed
/// lines and continue scanning so a healthy toolchain is still recognised.
///
/// PATTERN-1 / TASK-1566: also require the returned token to *look like* a
/// toolchain identifier — i.e. carry at least one of `-`/`.`/`:` so it has
/// the shape of `stable-aarch64-apple-darwin`, `1.70.0-…`, or
/// `linked:custom-toolchain`. Without this guard, rustup ≥1.28's
/// `"no active toolchain configured\n"` output (no `error:` prefix) would
/// surface as `Some("no")`, which then flows downstream to
/// `rustup component add <component> --toolchain no` producing a confusing
/// rustup error instead of the operator-facing "no active toolchain
/// configured" diagnostic. Real toolchain identifiers always carry one of
/// these separators; bare status words like `no` / `none` / `unknown` do
/// not.
pub(crate) fn parse_active_toolchain(stdout: &str) -> Option<String> {
    const RUSTUP_DIAGNOSTIC_PREFIXES: &[&str] = &["error:", "warning:", "info:", "note:"];

    stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .find_map(|line| {
            let token = line.split_whitespace().next()?;
            if RUSTUP_DIAGNOSTIC_PREFIXES.contains(&token) {
                return None;
            }
            // PATTERN-1 / TASK-1566: real toolchain identifiers always carry
            // a `-` (target-triple form), `.` (version-pinned form), or `:`
            // (linked-toolchain form / Windows path). Reject bare status
            // words like `no`, `none`, `unknown` so rustup ≥1.28's
            // `"no active toolchain configured\n"` output does not surface as
            // `Some("no")`.
            if !token.contains(['-', '.', ':']) {
                return None;
            }
            Some(token.to_string())
        })
}

/// API / TASK-1200: returns [`ProbeOutcome::Failed`] when the underlying
/// `rustup component list --installed` invocation cannot be answered
/// (timeout / IO / non-zero exit) so callers route it to
/// [`crate::ToolStatus::ProbeFailed`] instead of mis-reporting the
/// component as not installed.
pub fn check_rustup_component_installed(component: &str) -> ProbeOutcome<bool> {
    let mut cmd = Command::new(resolve_rustup_bin());
    cmd.args(["component", "list", "--installed"]);
    match run_probe_capturing(&mut cmd, "rustup component list --installed") {
        ProbeOutcome::Ok(stdout) => ProbeOutcome::Ok(is_component_in_list(&stdout, component)),
        ProbeOutcome::Failed => ProbeOutcome::Failed,
    }
}

/// `-{arch}-` patterns used to find the component-name / target-triple
/// boundary in lines like `clippy-preview-aarch64-apple-darwin`.
const RUSTUP_TARGET_ARCH_PATTERNS: &[&str] = &[
    "-aarch64-",
    "-arm-",
    "-armv6-",
    "-armv7-",
    "-armv7a-",
    "-asmjs-",
    "-i586-",
    "-i686-",
    "-loongarch64-",
    "-mips-",
    "-mips64-",
    "-mips64el-",
    "-mipsel-",
    "-nvptx64-",
    "-powerpc-",
    "-powerpc64-",
    "-powerpc64le-",
    "-riscv32-",
    "-riscv64-",
    "-s390x-",
    "-sparc-",
    "-sparc64-",
    "-thumbv6m-",
    "-thumbv7em-",
    "-thumbv7m-",
    "-thumbv7neon-",
    "-thumbv8m.base-",
    "-thumbv8m.main-",
    "-wasm32-",
    "-wasm64-",
    "-x86_64-",
];

fn strip_target_triple(line: &str) -> &str {
    for pat in RUSTUP_TARGET_ARCH_PATTERNS {
        if let Some(idx) = line.find(pat) {
            return &line[..idx];
        }
    }
    line
}

pub(crate) fn is_component_in_list(stdout: &str, component: &str) -> bool {
    let base = component.strip_suffix("-preview").unwrap_or(component);
    stdout.lines().any(|raw| {
        let line = raw.trim();
        let head = line.split_whitespace().next().unwrap_or(line);
        let stripped = strip_target_triple(head);
        stripped == base || stripped.strip_suffix("-preview") == Some(base)
    })
}

/// Capture the raw stdout of `rustup component list --installed` once.
pub fn capture_rustup_components() -> ProbeOutcome<String> {
    let mut cmd = Command::new(resolve_rustup_bin());
    cmd.args(["component", "list", "--installed"]);
    run_probe_capturing(&mut cmd, "rustup component list --installed")
}
