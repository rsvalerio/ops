---
id: TASK-2086
title: 'PERF-3: run and run_raw walk the command stores twice, ignoring expand_to_leaves_with_flags'
status: Done
assignee: []
created_date: '2026-09-07 22:58'
updated_date: '2026-09-09 18:15'
labels:
  - code-review-rust
  - performance
dependencies: []
parent_task_id: 'TASK-2244'
modified_files:
  - crates/runner/src/command/mod.rs
  - crates/runner/src/command/sequential.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:494-520`; `crates/runner/src/command/sequential.rs:114-124`

**What**: Both top-level entry points resolve the root command twice. `CommandRunner::run` calls `self.resolve(command_id)` to read the root composite's `parallel` / `fail_fast`, then calls `self.expand_to_leaves(command_id)` — whose implementation (`expand_to_leaves_with_flags`) already computes `(any_parallel, fail_fast_disabled)` in the same walk and throws the flags away. `run_raw` repeats the identical shape (`expand_to_leaves` + a second `resolve` for `fail_fast`). TASK-1283 built the flags-returning variant precisely so callers would not walk the tree twice, and its doc says "two independent traversals can drift in cycle/order semantics".

**Why it matters**: the redundant `resolve` is not just a wasted store walk per invocation — it is a second source of truth for the scheduling decision. `run` matches on the root spec's own flags while `expand_to_leaves_with_flags` aggregates them (uniform across the plan only because TASK-1657's agreement check enforces it); if that enforcement ever relaxes, the two reads can disagree silently. The flags from the single walk are exactly equivalent today (Exec root → any_parallel=false, fail_fast_disabled=false → the `run_plan(plan, true)` arm; composite root → uniform values), so switching is behaviour-preserving.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 run and run_raw obtain the plan and the scheduling flags from one expand_to_leaves_with_flags call and drop the separate resolve() of the root
- [x] #2 Existing behaviour is pinned: single-exec and composite plans, parallel and fail_fast variants, and the unknown-command error path keep their current event/result shapes (existing tests cover these and must pass unmodified)
- [x] #3 ops verify / ops qa gates pass

<!-- AC:END -->
