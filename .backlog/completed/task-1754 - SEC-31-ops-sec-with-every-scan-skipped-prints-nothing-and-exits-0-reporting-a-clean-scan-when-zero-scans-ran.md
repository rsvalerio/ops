---
id: TASK-1754
title: >-
  SEC-31: ops sec with every scan skipped prints nothing and exits 0, reporting
  a clean scan when zero scans ran
status: Done
assignee:
  - TASK-1982
created_date: '2026-08-27 11:16'
updated_date: '2026-08-28 19:23'
labels:
  - code-review-rust
  - security
dependencies: []
modified_files:
  - crates/cli/src/sec_cmd.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/sec_cmd.rs:427-442` (`run_sec_to`, execute branch)

**What**:

```rust
let mut all_ok = true;
for scan in selected {
    if !run_scan(root, scan, w)? { all_ok = false; }
}
Ok(if all_ok { ExitCode::SUCCESS } else { ExitCode::FAILURE })
```

When `selected` is empty the loop body never runs, `all_ok` stays `true`, and the command exits 0 having produced **no output at all** — not even a line saying nothing ran. `ops sec --skip secrets --skip vuln --skip misconfig` reaches this state: `build_plan` marks all three `(false, "skipped (--skip)")`, `selected` is empty, the `trivy_on_path()` guard still passes, and the command silently reports success.

`ops sec` is the terminal step of `ops qa` (AGENTS.md) and its exit code is what a CI gate or pre-push hook keys off. An empty-selection run is therefore indistinguishable, to every automated consumer, from "all scans ran and found nothing" — the security gate is inert and reports healthy.

The crate already rejects exactly this shape one module over, and says why. `run_cmd/plan.rs:1356-1366`:

```rust
if names.is_empty() {
    anyhow::bail!(
        "merge_plan called with empty names slice — refusing to plan zero commands \
         (this would otherwise execute zero steps and report success, masking an \
         upstream filtering bug)"
    );
}
```

`run_sec_to` has the same "executed zero steps, reported success" hazard with a higher blast radius, and no equivalent guard. Note the `--dry-run` branch is fine — it prints the full plan including the skip reasons, so the operator can see the state; only the execute branch is silent.

**Why it matters**: SEC-31 — a security check must fail closed. Silently succeeding with zero scans performed is the fail-open shape: a `.ops.toml`-driven or CI-config-driven skip triple turns the scan step into a no-op that no downstream signal distinguishes from a clean result.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 run_sec_to's execute branch handles an empty selected set explicitly instead of falling through the loop into SUCCESS
- [x] #2 The empty-selection outcome is visible to the operator: a line naming that zero scans ran and why (the skip reasons from the plan), on the same writer the scan lines use
- [x] #3 The exit code for an all-skipped run is decided deliberately and documented in the sec_cmd module doc and in the 'ops sec' clap help — either non-zero (fail closed, consistent with merge_plan's refusal) or 0 with the explicit no-scans line
- [x] #4 A unit test drives run_sec_to with every scan in --skip and asserts both the emitted message and the chosen exit code
<!-- AC:END -->
