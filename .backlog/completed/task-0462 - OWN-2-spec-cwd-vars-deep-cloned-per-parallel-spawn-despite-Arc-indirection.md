---
id: TASK-0462
title: 'OWN-2: spec/cwd/vars deep-cloned per parallel spawn despite Arc indirection'
status: Done
assignee:
  - TASK-0537
created_date: '2026-04-28 05:45'
updated_date: '2026-04-28 17:00'
labels:
  - code-review-rust
  - OWN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/parallel.rs:118` and `crates/runner/src/command/exec.rs:212-254`

**What**: spawn_parallel_tasks arc-wraps cwd/vars but the per-task closure does Arc::clone(&cwd)/Arc::clone(&vars) and passes them to exec_standalone, which passes them to exec_command, which passes &self.cwd/&self.vars. Inside exec_command, build_command_async clones spec, cwd.to_path_buf(), and vars.clone() — so the Arc indirection costs nothing on the hot path.

**Why it matters**: Variables likely contains a HashMap; cloning per spawn under MAX_PARALLEL=32 is non-trivial. Mixing Arc indirection with per-call deep clones is the worst of both.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 build_command_async signature takes Arc<Variables> (and optionally Arc<PathBuf>) so Arc::clone is the only allocation per spawn
- [x] #2 A microbenchmark (or tracing span on the spawn path) confirms ≤1 allocation for vars per spawn after the change
<!-- AC:END -->
