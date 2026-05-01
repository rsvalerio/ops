---
id: TASK-0778
title: >-
  FN-9: exec_standalone takes 6 positional parameters with overlapping types
  (id, spec, cwd, vars, tx, abort)
status: Done
assignee:
  - TASK-0824
created_date: '2026-05-01 05:57'
updated_date: '2026-05-01 09:57'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:316-323`

**What**: exec_standalone(id: CommandId, spec: ExecCommandSpec, cwd: Arc<PathBuf>, vars: Arc<Variables>, tx: mpsc::Sender<RunnerEvent>, abort: Arc<AbortSignal>) carries six parameters and triggers #[allow(clippy::too_many_arguments)]. FN-3 caps positional parameters at five.

**Why it matters**: The signature mixes per-task identity (id, spec) with shared infra (cwd, vars, tx, abort). Refactoring into an ExecTaskCtx struct with clone-on-spawn semantics would localise the cloning patterns and shrink the call site in spawn_parallel_tasks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Group the four shared Arc/Sender/AbortSignal parameters into an ExecTaskCtx struct with a single Clone impl, leaving (id, spec, ctx) as the public surface
- [x] #2 Drop the #[allow(clippy::too_many_arguments)] lint suppression
- [x] #3 Keep behaviour identical (no ownership/cloning regressions; pin with the existing parallel tests)
<!-- AC:END -->
