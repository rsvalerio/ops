---
id: TASK-1024
title: >-
  DUP-1: tests/integration.rs duplicate fork still exists after TASK-0939 (Done)
  — workspace-root file lacks the TEST-11/TEST-15/TEST-18/TEST-25 fixes applied
  to the cli-crate copy
status: Triage
assignee: []
created_date: '2026-05-07 20:22'
updated_date: '2026-05-07 20:23'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `tests/integration.rs:1-449` vs `crates/cli/tests/integration.rs:1-668`

**What**: TASK-0939 was marked Done on 2026-05-02 to dedupe these two integration test files, but the workspace-root `tests/integration.rs` still exists. It is the **older, weaker fork** of `crates/cli/tests/integration.rs` — `diff` reports 353 differing lines:

- `cli_run_unknown_command_fails` (root :117): asserts only `.failure()`. The cli-crate copy (TASK-0954 / TEST-11) asserts the stderr cites `unknown command: nonexistent_command`.
- `cli_run_with_malformed_toml` (root :286): asserts only `.failure()`. The cli-crate copy asserts `failed to parse config file: .ops.toml` (TEST-11).
- `cli_with_invalid_ops_d_file` (root :430): asserts only `.failure()`. The cli-crate copy asserts `.ops.d/invalid.toml` is named (TEST-11).
- `cli_run_command_with_timeout` (root :253): still uses `sleep 3` vs `timeout_secs = 1` — the 3s/1s flake-prone form TASK-0953 (TEST-15) replaced with `tail -f /dev/null` in the cli-crate copy.
- `cli_run_parallel_composite_command` (root :223): asserts only `success()` + `Done in` — identical to the sequential composite test, verifies nothing parallel-specific. TASK-0955 (TEST-25) replaced this with a marker-file rendezvous in the cli-crate copy.
- `ops()` helper (root :48): no `HOME` / `XDG_CONFIG_HOME` / `OPS_*` isolation. TASK-0957 (TEST-18) added `isolated_home()` + env stripping in the cli-crate copy so a developer's `~/.ops.toml` cannot leak into integration runs.
- Stale `#![allow(deprecated)]` (root :23) — TASK-0939 AC #2 specifically called for removing it; still present.

**Aggravating factor — the file is unowned and never compiles.** The workspace root `Cargo.toml` declares only `[workspace]` (no `[package]` block), and no member crate has a `[[test]]` target pointing at `tests/integration.rs`. So the file is orphaned source: `cargo test --workspace` never builds it, every IDE / clippy run skips it, and the maintainer effort that produced TASK-0953/0954/0955/0957 fixes was applied only to the cli-crate copy because the root copy is invisible to the build system. From a backlog-bookkeeping angle this is worse than it looks — TASK-0939 was effectively closed by *deleting the build wiring* but not the file, which leaves the dead fork as a hazard for any future contributor who copies it back into a `[[test]]` block thinking it's the canonical surface.

**Why it matters**: the fork is a maintenance trap — six waves of test-quality work were filed and closed against assertions that exist only in the cli-crate copy, and a future contributor or AI agent that reads `tests/integration.rs` first will believe the *weaker* assertions are the canonical surface. The right fix is to delete the file so the canonical home (`crates/cli/tests/integration.rs`) is the only thing the repo says exists.

Recommended action: delete `tests/integration.rs` outright. After removal, run `cargo test -p ops --test integration` to confirm the cli-crate copy still runs, and `ops verify`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 tests/integration.rs is removed (TASK-0939 AC #1 finally satisfied)
- [ ] #2 No remaining file in the repo carries a stale '#![allow(deprecated)]' for set_var/remove_var that is never used
- [ ] #3 Integration coverage runs only via crates/cli/tests/integration.rs (verified by cargo test -p ops --test integration)
- [ ] #4 ops verify / ops qa stay green after the deletion
<!-- AC:END -->
