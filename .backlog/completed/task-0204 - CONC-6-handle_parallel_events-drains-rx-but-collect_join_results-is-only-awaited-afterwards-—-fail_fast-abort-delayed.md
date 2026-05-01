---
id: TASK-0204
title: >-
  CONC-6: handle_parallel_events drains rx but collect_join_results is only
  awaited afterwards — fail_fast abort delayed
status: Done
assignee: []
created_date: '2026-04-22 21:27'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - CONC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:516-566` (handle_parallel_events + run_plan_parallel).

**What**: `run_plan_parallel` does: spawn_parallel_tasks → handle_parallel_events (drains rx to completion) → collect_join_results (then drains JoinSet). On fail_fast, `handle_parallel_events` sets `abort.store(true)` inside the loop but continues draining rx until every tx is dropped. Running tasks still emit `StepStarted`/`StepOutput` events after the abort flag is set, and the display keeps rendering them — so "fail fast" is "fail fast on the abort flag, but keep rendering until every task finishes or notices".

**Why it matters**: CONC-6 (structured concurrency). The current design keeps orphaned tasks running and keeps rendering their output, defeating the user-facing intent of fail_fast. Fix: use `tokio::select!` between `rx.recv()` and `join_set.join_next()` so we can break out on first failure and then `join_set.abort_all()`. Or at minimum, check `abort.load(Ordering::Acquire)` at the top of `exec_standalone`'s main body (not just at entry) to cancel in-flight children via their `kill_on_drop(true)` handles.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 run_plan_parallel with fail_fast aborts running tasks within ~1s of first failure
- [ ] #2 Add test that fail_fast stops a 5s sibling task after a 100ms failure
<!-- AC:END -->
