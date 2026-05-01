---
id: TASK-0774
title: >-
  PERF-3: rebuild_alias_map fully re-iterates stack + extension stores on every
  register_commands call
status: To Do
assignee:
  - TASK-0825
created_date: '2026-05-01 05:56'
updated_date: '2026-05-01 06:18'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:144-148`

**What**: register_commands accepts an iterator of (id, spec) and after the insert loop calls rebuild_alias_map, which iterates both stores and re-allocates the entire non_config_alias_map. If a runner is built incrementally with N successive register_commands calls of one entry each, total alias-map work is O(N · (|stack| + |extensions|)).

**Why it matters**: Extensions register commands one-batch-per-extension today, so N is small. Cost is amortised, but current implementation makes incremental registration quadratic if the call pattern ever shifts.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Update rebuild_alias_map (or add an incremental variant) to merge only the new entries into the existing map rather than rebuilding from scratch
- [ ] #2 Preserve existing collision-warning behaviour
- [ ] #3 Add a microtest registering N batches of 1 entry and asserting alias-map work scales linearly
<!-- AC:END -->
