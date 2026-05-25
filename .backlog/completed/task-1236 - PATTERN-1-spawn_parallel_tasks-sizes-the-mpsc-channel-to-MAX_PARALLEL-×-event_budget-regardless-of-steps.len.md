---
id: TASK-1236
title: >-
  PATTERN-1: spawn_parallel_tasks sizes the mpsc channel to MAX_PARALLEL ×
  event_budget regardless of steps.len()
status: Done
assignee:
  - TASK-1270
created_date: '2026-05-08 12:59'
updated_date: '2026-05-10 17:02'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/parallel.rs:207-210`

**What**: `capacity = max_parallel.saturating_mul(event_budget)` — with defaults that is 32 × 256 = 8192 slots. For a 2-step parallel plan the channel reserves the same capacity as a 32-step plan, even though only `min(steps.len(), max_parallel)` producers can ever be active. `mpsc::channel(capacity)` pre-allocates internal storage proportional to capacity.

**Why it matters**: Embedders driving many small parallel plans pay the full worst-case sizing per plan. The bound is wrong: maximum live producers is `min(steps.len(), max_parallel)`, not `max_parallel`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cap capacity at min(steps.len(), max_parallel).saturating_mul(event_budget)
- [ ] #2 Add a debug log of the chosen capacity
- [ ] #3 Regression test pinning the new sizing for a small parallel plan
<!-- AC:END -->
