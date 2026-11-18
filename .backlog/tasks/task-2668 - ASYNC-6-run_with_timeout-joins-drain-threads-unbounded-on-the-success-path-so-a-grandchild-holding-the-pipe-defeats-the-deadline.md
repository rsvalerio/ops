---
id: TASK-2668
title: 'ASYNC-6: run_with_timeout joins drain threads unbounded on the success path, so a grandchild holding the pipe defeats the deadline'
status: To Do
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - ASYNC
dependencies: []
parent_task_id: 'TASK-2840'
modified_files:
  - crates/core/src/subprocess/mod.rs
  - crates/core/src/subprocess/drain.rs
priority: medium
ordinal: 1000
dedup_key: 'ASYNC-6:crates/core/src/subprocess/mod.rs:run_with_timeout_inner'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess/mod.rs:269-270` (`run_with_timeout_inner`), `crates/core/src/subprocess/drain.rs:125-198` (`collect_drain`)

**What**: When the child exits before the deadline, `run_with_timeout_inner` calls `collect_drain(stdout_handle, ..)` / `collect_drain(stderr_handle, ..)`, which do an unbounded `JoinHandle::join()` on the pipe-drain threads. A drain thread only finishes at EOF, and EOF only arrives once *every* holder of the pipe's write end has closed it. A grandchild the child forked and left running (a daemonized helper, a `sh -c` that forks instead of exec'ing, a build-script server) inherits stdout/stderr and keeps the pipe open, so the join blocks for the grandchild's whole lifetime — after `wait_timeout` already returned `Some(status)`.

The timeout path already knows about this exact hazard: `drain_after_timeout` bounds its wait with `DRAIN_GRACE` and documents that "killing the child does not guarantee EOF on its pipes: any surviving grandchild inherits the write ends". The success path has the same exposure and no bound.

**Why it matters**: `run_with_timeout` / `run_cargo` promise bounded execution ("network-touching cargo subprocesses never hang indefinitely"). A successful child with a lingering grandchild makes the call hang past `timeout` with no error and no log line, which is the failure mode the helper exists to prevent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After the child exits, joining the drain threads is bounded by the remaining deadline (or a documented grace), never unbounded
- [ ] #2 A drain still blocked after that bound is abandoned with a tracing::warn naming label and stream, matching drain_after_timeout
- [ ] #3 A test spawns a child that leaves a grandchild holding stdout open and exits 0, and asserts run_with_timeout returns within the timeout
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.03 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
