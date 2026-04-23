---
id: TASK-0124
title: 'DUP-1: exec_command_raw duplicates timeout/status handling from exec_command'
status: Done
assignee: []
created_date: '2026-04-20 19:34'
updated_date: '2026-04-20 20:45'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:369-413` (`exec_command_raw`)

**What**: `exec_command_raw` reimplements the timeout wrapper, `Instant::now()` / `duration` bookkeeping, and the success/failure → `StepResult` conversion already present in `exec_command` (same file, a few hundred lines above). The two diverge only in: (a) stdio configuration (`inherit` vs `piped`), (b) no child-stdout/stderr reader tasks, and (c) no `RunnerEvent` emission. Any future change to timeout semantics, error shaping, or `StepResult::failure` payload has to be made twice.

**Why it matters**: Duplicated execution plumbing in a runner is exactly where drift silently produces inconsistent user-visible behavior (e.g. one path reporting `timed out after Ns`, the other using a different error message; one path recording duration including spawn time, the other not). The bug class is inherent to the duplication, not any specific current divergence.

**Suggested shape**: extract a private helper like `async fn run_to_status(cmd: tokio::process::Command, timeout: Option<Duration>) -> (Result<ExitStatus, io::Error>, Duration)` that both `exec_command` and `exec_command_raw` call; keep stdio wiring and event emission at the call sites.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Timeout and duration bookkeeping exist in one place shared by exec_command and exec_command_raw
- [ ] #2 Timeout error message ("timed out after Ns") is produced by shared code, not duplicated
- [ ] #3 Existing tests for both paths still pass; no behavior change for non-raw callers
<!-- AC:END -->
