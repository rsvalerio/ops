---
id: TASK-0206
title: >-
  CL-3: CommandRunner::run and expand_to_leaves both look up the spec
  separately, duplicating resolve() calls and creating a TOCTOU-like window
status: Done
assignee: []
created_date: '2026-04-22 21:27'
updated_date: '2026-04-23 15:06'
labels:
  - rust-code-review
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:569-590` (CommandRunner::run).

**What**: `run` does three independent lookups of the same command id: (1) `self.resolve(command_id)` to get the `CommandSpec`, (2) `self.expand_to_leaves(command_id)` which internally re-resolves the same id plus all children, (3) a second `match spec` to decide parallel vs sequential. If `&self.config` is immutable during a single call (which it is), these lookups are redundant; if a future refactor makes config mutable, the three lookups can observe different states (CL-3 implicit-assumption / race-like anti-pattern).

**Why it matters**: CL-3 ("make preconditions explicit"). Restructure so a single call returns `(ExpandedPlan, ExecutionPolicy)` where ExecutionPolicy encodes parallel/fail_fast/sequential-only. Bonus: the "`_ => self.run_plan(&plan, true, on_event).await`" fallback silently coerces non-composite single commands to fail_fast=true sequential without documenting that choice — encoding it in a named `ExecutionPolicy::SingleExec { fail_fast: true }` variant makes the intent explicit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Refactor CommandRunner::run to a single resolve+expand step returning an ExecutionPolicy enum
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Deferred: ExecutionPolicy refactor of CommandRunner::run is architectural and would touch the test surface heavily. The three lookups the task flags are cheap (HashMap) and the implicit-state concern is not reachable today because &self.config is immutable during run(). Document here for a future wave.
<!-- SECTION:NOTES:END -->
