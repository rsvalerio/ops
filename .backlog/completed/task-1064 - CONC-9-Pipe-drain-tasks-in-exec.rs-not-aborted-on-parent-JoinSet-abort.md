---
id: TASK-1064
title: 'CONC-9: Pipe-drain tasks in exec.rs not aborted on parent JoinSet abort'
status: Done
assignee: []
created_date: '2026-05-07 21:17'
updated_date: '2026-05-08 00:00'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:128-130`

**What**: `tokio::spawn(read_capped(stdout, cap))` spawns reader tasks held by `JoinHandle` via `.await`. When the outer parallel task is `JoinSet::abort_all()`'d during a hung child, these spawned drain tasks are not cancelled and keep reading until the child finally exits.

**Why it matters**: Defeats the fail-fast cancellation path (CONC-6/CONC-9 invariants). A wedged child plus an aborted parent leaves drain tasks alive consuming bytes and holding pipes open.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace bare tokio::spawn with JoinSet-owned spawns so Drop aborts them, OR thread an AbortSignal and select against it on the read
- [x] #2 Regression test: a hung child plus parent abort returns within bounded wall-clock and leaves no drain tasks running
<!-- AC:END -->
