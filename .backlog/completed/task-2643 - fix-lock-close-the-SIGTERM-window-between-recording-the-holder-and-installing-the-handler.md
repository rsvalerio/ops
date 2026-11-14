---
id: TASK-2643
title: 'fix(lock): close the SIGTERM window between recording the holder and installing the handler'
status: Done
assignee: []
created_date: '2026-10-27 11:52'
updated_date: '2026-10-29 21:04'
labels:
  - bug
  - lock
  - flaky-test
dependencies: []
modified_files:
  - crates/cli/src/lock_cmd.rs
  - crates/cli/tests/lock.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`ops lock run` writes the holder record (crates/cli/src/lock_cmd.rs:280, write_record) before it spawns the child (~:514) and registers its SIGINT/SIGTERM handlers in forward_signals_until_exit (~:524). A signal that arrives in that gap hits the default disposition: ops dies by signal (no exit code, not 128+15), the child is never started or is orphaned, and the record is left behind as stale instead of being cleared.

The integration test lock_is_shared_across_worktrees_and_released_on_sigterm (crates/cli/tests/lock.rs:130) hits this intermittently in CI: it sends SIGTERM as soon as the contender sees the lock held, and gets exit None instead of Some(143). It failed twice on PR #105 (unrelated change) and passed on re-run.

Fix direction: install the signal handlers before the lock is acquired/recorded (or block the signals until they are), so a signal at any point after the record exists is handled: child forwarded or not yet spawned, record cleared, exit 128+signal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A SIGINT/SIGTERM delivered at any point after the holder record is written makes ops exit 128+signal and clears the record
- [x] #2 If the signal arrives before the child is spawned, the child is not started
- [x] #3 lock_is_shared_across_worktrees_and_released_on_sigterm no longer flakes; a test sends the signal as early as possible (e.g. right after the record appears) and asserts the exit code and a free lock

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
SIGINT/SIGTERM are now caught (tokio signal streams in a Signals struct) before the lock is taken, inside the one current-thread runtime run_locked builds. acquire is async: a held lock is polled every 100ms (no blocking lock(), so a signal can end the wait without leaving a thread that might still take the lock) and a signal while waiting returns Acquired::Signalled with nothing recorded. A signal that arrived after the lock was taken but before the child is spawned drops the guard (record cleared) without starting the command. All three paths exit 128+signo. New tests in crates/cli/tests/lock.rs: sigterm_right_after_the_lock_is_taken_still_releases_it polls the record file directly and signals at once, 20 rounds — it fails every run on the old code (exit None) and passes with the fix; a_signal_while_waiting_ends_the_wait_without_running_the_command. ops verify qax 12/12.

Shipped in PR #117 (fix/lock-signal-window, v0.88.1); status closed 2026-10-29.

<!-- SECTION:NOTES:END -->
