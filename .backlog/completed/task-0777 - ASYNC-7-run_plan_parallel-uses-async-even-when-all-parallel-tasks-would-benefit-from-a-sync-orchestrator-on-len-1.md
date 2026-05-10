---
id: TASK-0777
title: >-
  ASYNC-7: run_plan_parallel uses async even when all parallel tasks would
  benefit from a sync orchestrator on len<=1
status: Done
assignee:
  - TASK-0824
created_date: '2026-05-01 05:57'
updated_date: '2026-05-01 09:55'
labels:
  - code-review-rust
  - async
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/parallel.rs:153`

**What**: Sequential path (run_plan) and raw path are both async because exec_command is async. The parallel path adds a forwarder per task, abort-signal racing, and a JoinSet — substantial async machinery that pays off only with true I/O concurrency. For small parallel groups (1–2 commands) the orchestration overhead exceeds the spawn savings.

**Why it matters**: ASYNC-7: use async when async adds value. A short-circuit that delegates to run_plan when command_ids.len() <= 1 would avoid the channel allocation, JoinSet, AbortSignal, and forwarder spawn per trivial parallel group.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add a fast-path in run_plan_parallel for command_ids.len() <= 1 that delegates to the sequential path
- [x] #2 Keep observable behaviour (events emitted, results returned) identical, with a regression test pinning the event order
- [x] #3 Document the threshold rationale near MAX_PARALLEL
<!-- AC:END -->
