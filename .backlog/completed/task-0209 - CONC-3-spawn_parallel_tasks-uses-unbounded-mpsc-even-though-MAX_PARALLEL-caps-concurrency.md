---
id: TASK-0209
title: >-
  CONC-3: spawn_parallel_tasks uses unbounded mpsc even though MAX_PARALLEL caps
  concurrency
status: Done
assignee: []
created_date: '2026-04-23 06:32'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - concurrency
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:494`

**What**: Unbounded channel funnels all task output into the same unbounded rx; chatty children still buffer unbounded RunnerEvents.

**Why it matters**: Memory growth is bounded only by subprocess output volume — a noisy command can OOM the process.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace mpsc::unbounded_channel() with mpsc::channel(capacity) sized from MAX_PARALLEL × per-task event budget
- [ ] #2 Document the capacity rationale in a comment near the constant
<!-- AC:END -->
