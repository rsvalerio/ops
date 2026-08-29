---
id: TASK-2023
title: >-
  CONC-14: after the shutdown path fires, a second Ctrl-C cannot force-quit a
  wedged teardown
status: To Do
assignee:
  - TASK-2043
created_date: '2026-08-28 19:42'
updated_date: '2026-08-29 11:35'
labels:
  - code-review-rust
  - concurrency
dependencies: []
modified_files:
  - crates/cli/src/run_cmd.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs` (`run_until_signal`)

**What**: TASK-1932 installs tokio signal handlers for `SIGTERM` and `SIGINT` so a signal cancels the plan instead of killing the process outright. Installing a handler replaces the default disposition for the rest of the process's life, so once `run_until_signal` has returned, further `SIGINT`/`SIGTERM` deliveries are consumed by tokio's (now unread) handler rather than terminating the process.

That removes the escape hatch users rely on. If teardown itself is slow or wedged — a child ignoring `SIGTERM` through its `GROUP_TERM_GRACE` window, a drain still running, the runtime waiting on a blocking-pool task — the operator's second Ctrl-C does nothing, where before TASK-1932 it would have killed `ops` immediately. The conventional shape is: first signal requests a graceful cancel, second signal exits hard.

Fix direction: keep the signal streams alive past the first trip and, on a second delivery, `std::process::exit(128 + signo)` after a best-effort terminal restore; or reset the disposition to default (`SIG_DFL`) once the cancel has been requested.

**Why it matters**: the graceful path is a strict improvement only if the user can still escape it. Making Ctrl-C ineffective during a slow shutdown is the exact frustration the previous behaviour did not have.

**Origin**: discovered during TASK-1986 while fixing TASK-1932.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 a second SIGINT/SIGTERM after the shutdown path has been entered terminates the process promptly with 128+signo
- [ ] #2 the terminal echo state is still restored on that hard-exit path, or the trade-off is documented where EchoGuard's Drop paths are listed
- [ ] #3 a test or documented manual procedure covers the double-signal case
<!-- AC:END -->
