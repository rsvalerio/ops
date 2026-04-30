---
id: TASK-0723
title: 'PERF-12: ProgressState::step_index linear scans bars Vec on every RunnerEvent'
status: To Do
assignee:
  - TASK-0741
created_date: '2026-04-30 05:32'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/progress_state.rs:54-56`

**What**: `step_index` does `self.steps.iter().position(|(sid, _)| sid == id)`, an O(n) linear scan over the per-plan step Vec. Every `RunnerEvent` (Started/Output/Finished/Failed/Skipped/Dropped) routes through `ProgressDisplay::handle_event` which calls `step_index` to translate id -> bar slot, so a parallel plan with N steps emitting M output lines each performs N*M linear lookups. With `MAX_PARALLEL=32` and a chatty cargo test (hundreds of stderr lines per step), the inner loop is O(N) per output event when an O(1) HashMap<id, usize> index would suffice.

**Why it matters**: PERF-12 (composed map for O(1)+O(log n) lookup) — the index pattern is exactly what the rule prescribes, and the rendering hot path runs synchronously on the event-pump thread. The current cost is acceptable for typical N<10 plans but degrades quadratically once large parallel groups land. The fix is mechanical: alongside `steps: Vec<(String, String)>` keep `index_by_id: HashMap<String, usize>` populated by `reset_for_plan`. Memory cost is one usize+String per step.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ProgressState carries an HashMap<String, usize> id index populated in reset_for_plan; step_index hits the map instead of scanning the Vec
- [ ] #2 Existing tests pass; one new test asserts that step_index on a 32-step plan after 1000 mixed events does not scan the steps Vec (e.g. via a counter wrapped around the lookup)
<!-- AC:END -->
