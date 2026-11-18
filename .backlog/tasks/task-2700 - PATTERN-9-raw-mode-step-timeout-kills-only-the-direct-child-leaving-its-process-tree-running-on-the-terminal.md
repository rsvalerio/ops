---
id: TASK-2700
title: 'PATTERN-9: raw-mode step timeout kills only the direct child, leaving its process tree running on the terminal'
status: To Do
assignee: []
created_date: '2026-11-18 18:31'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
parent_task_id: 'TASK-2840'
modified_files:
  - crates/runner/src/command/exec.rs
  - crates/runner/src/command/process_group.rs
priority: medium
ordinal: 1000
dedup_key: 'PATTERN-9:crates/runner/src/command/exec.rs:exec_command_raw'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:687` (`exec_command_raw`), relying on `cmd.kill_on_drop(true)` at `crates/runner/src/command/build.rs:692`

**What**: `exec_command_raw` runs `run_with_timeout(cmd.status(), spec.timeout())`. When `timeout_secs` elapses the `status()` future is dropped and the only cleanup is tokio's `kill_on_drop`, which sends `SIGKILL` to the **direct child pid**. Captured steps were fixed for exactly this (`process_group.rs`: own process group + `ChildGroup` SIGTERM/SIGKILL of the whole tree), but raw steps deliberately stay in the runner's process group for Ctrl-C delivery and get no tree teardown at all. A raw step such as `sh -c "cargo build"` or `npm run dev` that times out has its shell killed while `cargo`/`rustc`/`node` keep running, unowned, still attached to the user's terminal and still holding `.cargo-lock`, after `ops` has reported the step failed and moved on (or exited). The same applies per cell in `run_matrix_raw`.

**Why it matters**: The `process_group.rs` module docs describe this leak as the reason the captured path owns the tree; the raw path documents only the Ctrl-C trade-off and silently keeps the leak on timeout. Orphans keep writing over the next step's output and can block the next build on the cargo lock. The cleanup is a Drop-only guard that cannot reach the work it is meant to stop (PATTERN-9: cleanup whose absence is a correctness problem needs more than the guard).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A raw-mode step that exceeds timeout_secs no longer leaves descendants running (e.g. raw children get their own process group made foreground with tcsetpgrp, or the timeout path signals the descendant tree), or the limitation is documented on exec_command_raw and in user-facing timeout docs as a deliberate decision
- [ ] #2 A unix test runs a raw step 'sh -c "sleep 30 & wait"' (or equivalent) with a short timeout and asserts the grandchild is gone after the step returns, or the decision is recorded
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.05 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
