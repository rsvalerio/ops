---
id: TASK-1919
title: >-
  CONC-9: kill_on_drop only reaches the direct child, so a timed-out or
  fail-fast-cancelled step orphans its whole process tree and can wedge the pipe
  drain
status: To Do
assignee:
  - TASK-1986
created_date: '2026-08-27 15:44'
updated_date: '2026-08-28 14:10'
labels:
  - code-review-rust
  - concurrency
dependencies: []
modified_files:
  - crates/runner/src/command/exec.rs
  - crates/runner/src/command/build.rs
  - crates/runner/src/command/parallel.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:626` (`cmd.kill_on_drop(true)`), `crates/runner/src/command/exec.rs:111-192` (`spawn_capped`), `crates/runner/src/command/exec.rs:429` (timeout wrapper), `crates/runner/src/command/parallel.rs:563,585` (`join_set.abort_all()`)

**What**: the runner's only cancellation mechanism for a running child is `kill_on_drop(true)`. Three paths rely on it:

- timeout: `run_with_timeout(spawn_capped(&mut cmd, cap), spec.timeout())` — `tokio::time::timeout` drops the `spawn_capped` future, which drops `child`;
- fail-fast: `handle_parallel_events_with_cancel_inner` calls `join_set.abort_all()`, dropping each task's `spawn_capped` frame;
- plan teardown on any early return.

`kill_on_drop` sends `SIGKILL` to the **direct child pid only**. It does not create or signal a process group. Every step in this codebase is a program that itself forks (`cargo build` → `rustc` × N, `npm run` → node → subprocesses, `sh -c "..."` → whatever it launched, `docker` → the daemon-side work). Killing `cargo` leaves the `rustc` fleet running; killing `sh` leaves its children running. Nothing ever reaps them and they keep consuming the CPU/RAM the timeout was supposed to reclaim. The existing tests demonstrate the shape — `run_plan_parallel_fail_fast_emits_terminal_for_every_started_step` (`command/tests/parallel.rs:137`) cancels an `sh -c "sleep 5"` and only ever proves the *event* arrived, never that the tree died.

The same missing group ownership produces a second, sharper failure: the orphaned grandchildren still hold the write end of the stdout/stderr pipes. In `spawn_capped` the drain tasks are joined **after** `child.wait().await` (`exec.rs:147-178`), and `read_capped` runs to EOF with no bound. So when the direct child exits but a grandchild it spawned (a daemonised watcher, a backgrounded `&` job in an `sh -c` step, a lingering `rustc`) retains the inherited pipe fd, `child.wait()` returns immediately and then `spawn_capped` blocks in `read_capped` forever. With `spec.timeout() == None` — the default, since `timeout_secs` is `Option<u64>` and unset in most configs — the step never completes and the whole plan hangs on a child that already exited.

The fix is to own the tree: `std::os::unix::process::CommandExt::process_group(0)` (exposed on `tokio::process::Command` via `as_std_mut`) so each spawn is its own group leader, then `killpg(pgid, SIGTERM)` with a short grace window and `killpg(pgid, SIGKILL)` on expiry, driven from an explicit cancel path rather than from `Drop`. That also gives the child a chance to clean up (a `SIGKILL`ed `cargo` leaves a stale `.cargo-lock`; a `SIGKILL`ed `docker build` leaves dangling layers), which the current `kill_on_drop`-only design cannot do at all.

**Why it matters**: this is the failure users actually hit — `ops` reports the step as timed out or cancelled and returns to the prompt while the machine stays pinned by a compiler fleet nobody owns, or the plan silently never finishes because an exited child's descendants still hold the pipe. Both are in the engine's cancellation path, which exists precisely to stop work.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 each spawn becomes its own process-group leader (process_group(0) on unix) so the runner can address the whole tree
- [ ] #2 timeout and fail-fast cancellation signal the process group (SIGTERM, then SIGKILL after a bounded grace period) instead of relying solely on kill_on_drop of the direct child
- [ ] #3 spawn_capped cannot block indefinitely in read_capped after child.wait() returns: the post-exit drain is bounded (deadline or explicit pipe close) so a grandchild holding the inherited pipe fd cannot hang the step
- [ ] #4 a regression test spawns a step whose child forks a longer-lived grandchild (e.g. sh -c 'sleep 30 & echo started'), cancels via timeout or fail_fast, and asserts both that the step returns promptly and that the grandchild is no longer running
- [ ] #5 the non-unix build path is documented or feature-gated; kill_on_drop remains the fallback where process groups are unavailable
<!-- AC:END -->
