---
id: TASK-2867
title: 'core/hooks: serialize piped spawns outside the runner so macOS pipe()+FD_CLOEXEC races cannot leak capture pipes'
status: Done
assignee: []
created_date: '2026-11-18 21:22'
updated_date: '2026-11-21 16:11'
labels:
  - code-review-rust
  - CONC
dependencies: []
parent_task_id: 'TASK-2880'
modified_files:
  - crates/runner/src/command/exec.rs
  - crates/core/src/subprocess/mod.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs` (`SPAWN_LOCK`), `crates/core/src/subprocess/mod.rs` (`run_with_timeout_inner`)

**What**: TASK-2644's root cause is std's non-atomic pipe creation on targets without `pipe2` (macOS): `pipe()` then `set_cloexec`, so a spawn on another thread inside that window inherits the write end, and the holder lives outside the step's process group. TASK-2840 added a process-wide `SPAWN_LOCK` around captured runner spawns only. Other piped spawn sites (`ops_core::subprocess::run_with_timeout` on blocking threads, hook and extension subprocesses) can still race with each other or with runner steps. Hoist the lock into `ops-core` and take it at every piped spawn, then confirm on macOS (the failure was never reproducible on Linux, where `pipe2` is atomic) that `ops verify` with parallel steps no longer logs "captured output still open after killing the step's process group".

**Why it matters**: A leaked write end delays EOF on another step's capture until the unrelated holder exits.

**Origin**: discovered during TASK-2840 while fixing TASK-2644.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 One process-wide spawn lock in ops-core is taken by every spawn that creates pipes
- [x] #2 A macOS run of ops verify with parallel steps is confirmed free of the held-pipe warning

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
wave28 (TASK-2880): AC#1 done. ops_core::subprocess::spawn_locked holds the one process-wide spawn lock, and output_locked is output() with the spawn taken under it. Callers that take it: run_with_timeout, the runner's spawn_serialized (still inside spawn_blocking, ChildGroup still armed on that thread), cargo clippy findings, the hook-common staged-files probe, the coderabbit preflight and review, text-fixers git ls-files, and terraform show -json. One spawn is left out: ops-backlog's git helper does not depend on ops-core. It is single-threaded and never runs alongside another spawn in its process, so it cannot open the window. Linux nextest is green for core, runner, cli, hook-common, coderabbit, text-fixers and tfplan.
AC#2 is still open. It needs a macOS run of ops verify with parallel steps that shows no "captured output still open after killing the step's process group" warning. No macOS host or CI job was available (CI has no macOS test job).

AC#2 confirmed on macOS (aarch64, 2026-11-21) with ops 0.94.6 built from main 9d45cf2a. (1) ops verify (parallel, 11 steps) run 6 times under Rust 1.98.0: all 11/11 passed, no 'captured output still open' or 'held open' line. (2) Spawn stress: a parallel command of 16 'sleep 8' steps interleaved with 48 'echo' steps, OPS_LOG_LEVEL=warn. Control ops 0.94.0 (no spawn lock): 3 of 55 runs logged the held-pipe WARN and failed a step with 'captured output pipe held open after the child exited; step abandoned', the original TASK-2644 symptom. ops 0.94.6: 0 of 150 runs. At the control's 2/30 rate, 150 clean runs by chance is about 3e-5.

<!-- SECTION:NOTES:END -->
