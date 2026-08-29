---
id: TASK-2023
title: >-
  CONC-14: after the shutdown path fires, a second Ctrl-C cannot force-quit a
  wedged teardown
status: Done
assignee:
  - TASK-2043
created_date: '2026-08-28 19:42'
updated_date: '2026-08-29 12:51'
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
- [x] #1 a second SIGINT/SIGTERM after the shutdown path has been entered terminates the process promptly with 128+signo
- [x] #2 the terminal echo state is still restored on that hard-exit path, or the trade-off is documented where EchoGuard's Drop paths are listed
- [x] #3 a test or documented manual procedure covers the double-signal case
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Fixed in code-review wave TASK-2043. run_until_signal now calls restore_default_shutdown_dispositions() inside each signal arm of the select! — before the losing plan future is dropped, so the teardown itself stays escapable — resetting SIGINT and SIGTERM to SIG_DFL via libc::signal. A second Ctrl-C/kill during a wedged teardown therefore terminates the process the way it would have pre-TASK-1932, which shells report as 128+signo (130/143), matching the graceful path. The spawn-a-watcher-task alternative was rejected: sequential plans run on a current_thread runtime, where a wedged synchronous drop starves any spawned task. libc moved from dev-dependencies to [target.cfg(unix).dependencies] in crates/cli. AC #2: the trade-off (this hard-exit path bypasses unwinding, so EchoGuard::drop does not run and ECHO stays cleared; `reset` is the remedy) is documented both on restore_default_shutdown_dispositions and as a new row plus paragraph in EchoGuard Drop-paths table in crates/runner/src/terminal.rs. AC #3: unit test shutdown_path_restores_default_signal_dispositions queries both dispositions with sigaction after the shutdown path, and a manual double-Ctrl-C procedure is documented on the function. The reset is process-global and not undoable in-process (tokio registers each OS handler behind a Once); the test notes it relies on the project process-per-test nextest harness, the same assumption the sibling SIGTERM test already makes.
<!-- SECTION:NOTES:END -->
