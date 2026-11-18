---
id: TASK-2644
title: 'runner: a passing step intermittently fails with ''captured output pipe held open after the child exited; step abandoned'''
status: Done
assignee: []
created_date: '2026-11-13 21:03'
updated_date: '2026-11-18 21:27'
labels:
  - runner
  - conc
dependencies: []
parent_task_id: 'TASK-2840'
modified_files:
  - crates/runner/src/command/exec.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs` (post-exit drain in the captured-step path; DEFAULT_POST_EXIT_DRAIN_GRACE_SECS = 5, POST_KILL_DRAIN_GRACE)

**What**: In the event0 repo (ops 0.85.1, macOS aarch64), `ops verify` steps intermittently fail with `captured output pipe held open after the child exited; step abandoned`, preceded by the WARN `captured output still open after the child exited; killing the process group grace_secs=5`. The step's own command succeeded; an immediate re-run passes. Observed on 2026-11-13 on `cargo build --workspace --all-features --all-targets --locked`, `ops check-json` and `ops check-yaml`, in at least 5 runs: four with 4 worktrees running `ops verify` concurrently, and one with nothing else running (so contention is not required).

**Ruled out**: the sccache server spawned by `rustc-wrapper = "sccache"` holding the pipe. With the server stopped, `cargo build` through a pipe closed the pipe the moment cargo exited.

**Open questions**: what descendant still holds the write end after the leader exits, especially for `ops check-json`, which runs in 0.02s. It also matters that the kill-then-drain path abandons the step instead of reporting its real exit status, which was 0. One option is to return the leader's status with the capture truncated and a warning, rather than failing a step whose command succeeded.

**Why it matters**: every affected run costs a re-run, a merge lock is held longer, and a verify that fails for reasons unrelated to the code teaches people to ignore red verifies.

**Origin**: moved from event0 TASK-1394.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Root cause of the held-open pipe identified, with a reproduction
- [x] #2 A step whose command exited 0 is not reported as failed solely because a descendant held the capture pipe past the drain grace

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Resolved in TASK-2840. Root cause: on targets without pipe2 (macOS) std creates capture pipes with pipe() then sets FD_CLOEXEC non-atomically (library/std/src/sys/pipe/unix.rs); a parallel step spawned in that window inherits the write end, and that holder sits in another process group, so the group SIGKILL cannot release it — consistent with macOS-only, parallel-step-only occurrences. Linux is immune (pipe2), so the race itself was not reproducible here; the symptom is reproduced by the test a_pipe_held_outside_the_group_keeps_the_exit_status_and_output (perl setpgrp holder). Fix: SPAWN_LOCK serializes captured spawns; a pipe still open after the post-kill grace now stops the drains and returns the step's real exit status with output captured so far (warn logged) instead of failing. Follow-up to cover other spawn sites and confirm on macOS: TASK-2867 (Triage).
<!-- SECTION:NOTES:END -->
