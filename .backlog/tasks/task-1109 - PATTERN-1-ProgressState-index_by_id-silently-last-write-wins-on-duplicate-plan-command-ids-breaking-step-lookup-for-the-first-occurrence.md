---
id: TASK-1109
title: >-
  PATTERN-1: ProgressState::index_by_id silently last-write-wins on duplicate
  plan command ids, breaking step lookup for the first occurrence
status: Done
assignee: []
created_date: '2026-05-07 21:46'
updated_date: '2026-05-08 06:31'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/progress_state.rs:114`

**What**: `reset_for_plan` builds `index_by_id` via `insert(sid.clone(), idx)` in a `for` loop over `self.steps`. When a plan repeats the same `CommandId` (legal in parallel groups — TASK-0997 documents that `expand_to_leaves` only guards cycles, not duplicates, and the parallel orchestrator now counts terminal events rather than dedup-by-HashSet), the second `insert` overwrites the first, so `step_index(id)` always returns the index of the *last* occurrence. The `bars` and `steps` arrays still carry separate slots for both occurrences, but every `RunnerEvent` for that id (StepStarted / StepOutput / StepFinished / StepFailed) is routed to the second bar via `step_index`, leaving the first bar permanently in its initial state and double-rendering progress on the second.

**Why it matters**: Sibling task TASK-0997 explicitly fixed the orphan-skip synthesizer to count occurrences instead of dedup'ing by HashSet, but the display-side bookkeeping silently re-introduces the same hazard one layer down. A composite that fans the same leaf twice now under-reports progress on the parallel display: the first row sits as "pending" forever while the second row gets two StepStarted updates. The existing test `step_index_resolves_via_o1_map_for_large_plan` does not exercise duplicates and the regression is invisible on single-id plans. Fix: store a `Vec<usize>` (or a `HashMap<String, Vec<usize>>`) of indices per id and have the event router consume one index per terminal event, mirroring the TASK-0997 occurrence-counting pattern; alternatively, audit whether duplicates are reachable from CLI-level callers and, if not, surface a `tracing::warn!` plus reject the duplicate at `reset_for_plan` to keep the invariant explicit.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ProgressState::reset_for_plan no longer silently last-write-wins on duplicate command ids; either each occurrence has a distinct routable index or duplicates are rejected/warned at reset time
- [x] #2 Regression test fails on the current implementation: a plan with a duplicated id receives StepStarted/StepFinished on each occurrence and both bars reach a terminal state
<!-- AC:END -->
