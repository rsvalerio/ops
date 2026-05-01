---
id: TASK-0767
title: >-
  READ-5: collect_join_results loses the original CommandId on a panicking
  parallel task
status: Done
assignee:
  - TASK-0824
created_date: '2026-05-01 05:55'
updated_date: '2026-05-01 09:47'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/parallel.rs:71-77`

**What**: When a parallel task panics, the result is constructed as StepResult::failure("<panicked>", Duration::ZERO, "task panicked"). The actual CommandId is captured only in a tracing::debug! line; StepResult vector returned to the caller carries the sentinel id.

**Why it matters**: Programmatic consumers (CLI tests, JSON event consumers, CI dashboards) walking StepResult see "<panicked>" with no way to correlate. JoinSet::join_next_with_id (stable in tokio) would let us preserve the id.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Use JoinSet::join_next_with_id (or thread the CommandId via a wrapping spawn future) so panicked-task StepResults carry their real id
- [x] #2 Keep SEC-21 redaction of the panic payload (no payload string in the StepResult message)
- [x] #3 Regression test: spawn a parallel task that panics, assert the returned StepResult.id matches the originating CommandId
<!-- AC:END -->
