---
id: TASK-1177
title: >-
  CONC-6: handle_parallel_events_with_cancel only triggers fail_fast on
  StepFailed, not on panicked tasks
status: Done
assignee:
  - TASK-1261
created_date: '2026-05-08 08:08'
updated_date: '2026-05-08 14:51'
labels:
  - code-review-rust
  - conc
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/parallel.rs:399`

**What**: The fail_fast trigger inspects only `RunnerEvent::StepFailed`. A panicked task surfaces via `JoinSet::join_next` after the channel is drained, so `collect_join_results` synthesizes a failure result *after* `handle_parallel_events_with_cancel` has already returned. Under fail_fast, panicked siblings have already kept emitting until the channel closes naturally — abort never fires for the panic case.

**Why it matters**: A misbehaving task that panics rather than returning a non-zero exit code still gets the "drain everything until channel closes" treatment, defeating the fail_fast contract for the panic path. The orphan-skipped reconciliation in run_plan_parallel papers over this by walking command_ids after the fact, but live siblings continue running in the meantime.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When the JoinSet observes a panicked task (via join_next_with_id returning Err(JoinError) that is not a cancellation), abort is also set under fail_fast so live siblings stop.
- [ ] #2 Regression test panics one parallel task and asserts a sibling is aborted before completing its sleep.
<!-- AC:END -->
