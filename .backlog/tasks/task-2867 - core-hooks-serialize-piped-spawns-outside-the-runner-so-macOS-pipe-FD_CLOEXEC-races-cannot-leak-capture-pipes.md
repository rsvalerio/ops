---
id: TASK-2867
title: 'core/hooks: serialize piped spawns outside the runner so macOS pipe()+FD_CLOEXEC races cannot leak capture pipes'
status: To Do
assignee: []
created_date: '2026-11-18 21:22'
updated_date: '2026-11-19 07:57'
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
- [ ] #1 One process-wide spawn lock in ops-core is taken by every spawn that creates pipes
- [ ] #2 A macOS run of ops verify with parallel steps is confirmed free of the held-pipe warning
<!-- AC:END -->
