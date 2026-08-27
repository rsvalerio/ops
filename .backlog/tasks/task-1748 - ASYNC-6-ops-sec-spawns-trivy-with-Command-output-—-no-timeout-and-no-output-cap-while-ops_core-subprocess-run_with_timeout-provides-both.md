---
id: TASK-1748
title: >-
  ASYNC-6: ops sec spawns trivy with Command::output() — no timeout and no
  output cap, while ops_core::subprocess::run_with_timeout provides both
status: Triage
assignee: []
created_date: '2026-08-27 11:14'
labels:
  - code-review-rust
  - idioms
dependencies: []
modified_files:
  - crates/cli/src/sec_cmd.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/sec_cmd.rs:348-354` (`run_trivy`)

**What**:

```rust
fn run_trivy(root: &Path, scan: Scan) -> anyhow::Result<std::process::Output> {
    Command::new("trivy")
        .args(scan.trivy_args())
        .arg(root)
        .output()
        .with_context(|| format!("failed to run `trivy` for the {} scan", scan.label()))
}
```

`std::process::Command::output()` waits for the child **forever** and buffers **all** of its stdout/stderr in memory. Both are wrong for this callee:

- **No deadline.** `trivy fs --scanners vuln` downloads and refreshes its vulnerability database from a remote registry on first run and whenever the cached DB is stale. A slow or unreachable registry, a captive-portal proxy, or a hung TLS handshake leaves `ops sec` blocked with no output beyond the half-written `scanning vulnerabilities ` line (`run_scan` writes and flushes the prefix before spawning). AGENTS.md documents `ops qa` as ending in `sec`, so this is the terminal step of the project's own gate — a hang here is a hung CI job or a hung pre-push hook with no diagnostic.
- **No output cap.** The full report is read into a `Vec<u8>` before anything is printed. A `--scanners misconfig` run over a large IaC monorepo, or a secret scan that matches heavily, is bounded only by available memory.

This crate does not need to hand-roll either bound. `ops-core` is already a dependency and exports `ops_core::subprocess::run_with_timeout(cmd, timeout, label) -> Result<Output, RunError>` (`crates/core/src/subprocess/mod.rs:212`), which is the workspace's established answer: a single `wait_timeout` syscall (no polling), `kill` + drain on expiry, per-stream byte caps via `read_capped` (its own doc cites SEC-33/TASK-1050), a `RunError::Timeout` variant carrying the label and the deadline, and a `RunError::Spawn` that names the program. `extensions/hook-common/src/git_state.rs` already uses the same `wait_timeout` discipline for its git probes. `sec_cmd` is the outlier.

**Why it matters**: ASYNC-6 — every external call needs a timeout; SEC-33 — bound resource consumption when buffering output you do not control. The failure mode is an unbounded hang in the command that gates commits and CI, on a code path whose only job is to fail fast, and the fix is to call a helper that already exists in a crate this one already depends on.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_trivy goes through ops_core::subprocess::run_with_timeout (or an equivalent bounded wait) instead of Command::output(), passing a label that names the scan
- [ ] #2 A per-scan timeout is defined as a named constant with a comment justifying the value, and is overridable by the operator (config key or env var) since a first-run vulnerability-DB download is legitimately slow
- [ ] #3 A timeout is reported distinctly from a scan failure: the line ends with the failure marker and the message says the scan timed out after N seconds and names the escape hatch, rather than surfacing a bare RunError
- [ ] #4 A timed-out scan makes the aggregated ops sec exit code non-zero (fail closed), matching the findings path
- [ ] #5 A unit test drives the timeout branch with a short deadline against a blocking stand-in program and asserts both the message and the non-zero aggregate exit code
<!-- AC:END -->
