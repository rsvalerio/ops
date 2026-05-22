//! `rustup` toolchain / component probes.

use ops_core::subprocess::resolve_rustup_bin;
use std::collections::HashSet;
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
///
/// PATTERN-1 / TASK-1583 — drift strategy. rustup grows new target
/// architectures every few releases (loongarch64, riscv32im-…, etc.).
/// A hand-curated arch list silently goes stale: when a new triple
/// appears, [`strip_target_triple`] does not find a boundary, the line
/// is compared against `head` verbatim, and an installed component on
/// the new triple is reported as not-installed — triggering a redundant
/// `rustup component add`. We keep the list (rustup itself has no
/// machine-readable boundary marker and the candidate parses —
/// `rustup target list` output — are themselves part of what this list
/// must classify) but pair it with the
/// [`rustup_arch_patterns_cover_installed_targets`] regression test
/// below: for every triple `rustup target list` enumerates, the test
/// asserts our patterns recognise its architecture prefix. A new
/// rustup release that adds an arch we do not track fails CI here.
const RUSTUP_TARGET_ARCH_PATTERNS: &[&str] = &[
    "-aarch64-",
    "-arm-",
    "-arm64ec-",
    "-armv5te-",
    "-armv6-",
    "-armv7-",
    "-armv7a-",
    "-armv7r-",
    "-armv8r-",
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
    "-riscv32i-",
    "-riscv32im-",
    "-riscv32imac-",
    "-riscv32imafc-",
    "-riscv32imc-",
    "-riscv64-",
    "-riscv64a23-",
    "-riscv64gc-",
    "-riscv64imac-",
    "-s390x-",
    "-sparc-",
    "-sparc64-",
    "-sparcv9-",
    "-thumbv6m-",
    "-thumbv7em-",
    "-thumbv7m-",
    "-thumbv7neon-",
    "-thumbv8m.base-",
    "-thumbv8m.main-",
    "-wasm32-",
    "-wasm32v1-",
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

/// PERF-3 / TASK-1616: pre-tokenise `rustup component list --installed`
/// stdout into a hash set of stripped component heads (no target triple).
/// [`is_component_in_set`] is then O(1) per tool instead of an O(L) line
/// scan for each of T tools.
pub(crate) fn rustup_components_index(stdout: &str) -> HashSet<String> {
    stdout
        .lines()
        .filter_map(|raw| {
            let line = raw.trim();
            if line.is_empty() {
                return None;
            }
            let head = line.split_whitespace().next().unwrap_or(line);
            let stripped = strip_target_triple(head);
            if stripped.is_empty() {
                None
            } else {
                Some(stripped.to_string())
            }
        })
        .collect()
}

/// PERF-3 / TASK-1616: O(1) variant of [`is_component_in_list`] over a
/// precomputed [`rustup_components_index`] set. Preserves the
/// `-preview` reciprocity (`clippy` matches `clippy-preview` lines and
/// vice versa).
pub(crate) fn is_component_in_set(set: &HashSet<String>, component: &str) -> bool {
    let base = component.strip_suffix("-preview").unwrap_or(component);
    if set.contains(base) {
        return true;
    }
    let with_preview = format!("{base}-preview");
    set.contains(&with_preview)
}

#[cfg(test)]
mod rustup_arch_drift_tests {
    use super::*;

    /// PATTERN-1 / TASK-1583: every architecture rustup currently
    /// enumerates via `rustup target list` must be matched by one of
    /// [`RUSTUP_TARGET_ARCH_PATTERNS`]. A new rustup release that adds
    /// an architecture not in this list silently breaks
    /// [`is_component_in_list`] (an installed component on the new
    /// triple gets reported as not-installed, triggering a redundant
    /// install). Surfacing the drift in CI is cheaper than chasing the
    /// downstream symptom.
    #[test]
    fn rustup_arch_patterns_cover_installed_targets() {
        let mut cmd = Command::new(resolve_rustup_bin());
        cmd.args(["target", "list"]);
        let ProbeOutcome::Ok(stdout) = run_probe_capturing(&mut cmd, "rustup target list") else {
            eprintln!(
                "rustup_arch_patterns_cover_installed_targets: `rustup target list` did not \
                 answer; skipping"
            );
            return;
        };

        let mut missing: Vec<String> = Vec::new();
        for raw in stdout.lines() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }
            let Some(triple) = line.split_whitespace().next() else {
                continue;
            };
            let Some((arch, _)) = triple.split_once('-') else {
                continue;
            };
            let pattern = format!("-{arch}-");
            if !RUSTUP_TARGET_ARCH_PATTERNS.contains(&pattern.as_str())
                && !missing.contains(&pattern)
            {
                missing.push(pattern);
            }
        }

        assert!(
            missing.is_empty(),
            "rustup enumerates target triples whose arch prefix is not in \
             RUSTUP_TARGET_ARCH_PATTERNS; add them to src/probe/rustup.rs: {missing:?}"
        );
    }
}
