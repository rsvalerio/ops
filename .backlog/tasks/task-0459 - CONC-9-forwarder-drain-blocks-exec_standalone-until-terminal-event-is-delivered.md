---
id: TASK-0459
title: >-
  CONC-9: forwarder drain blocks exec_standalone until terminal event is
  delivered
status: To Do
assignee:
  - TASK-0537
created_date: '2026-04-28 05:45'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:371-374`

**What**: After exec_command returns, exec_standalone drains the forwarder JoinSet with `while forwarders.join_next().await.is_some() {}` then awaits `tx.send(terminal)`. If the outer bounded channel is full (display pump stalled), the terminal event awaits indefinitely. In fail_fast scenarios where parent calls `join_set.abort_all()`, exec_standalone is now stuck on tx.send.

**Why it matters**: Couples cancellation latency to display-pump latency. parallel.rs:174 says fail_fast should stop sibling output promptly, but a hung tx.send only completes when the outer receiver is dropped.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Terminal-event send in exec_standalone is wrapped in select! { _ = tx.send(...) => {}, _ = abort_signal => {} } (or equivalent timeout) so an aborted task does not block on a full outer channel
- [ ] #2 Test: with a deliberately-stalled mpsc::Receiver, an aborted parallel task completes within ~50ms instead of hanging for the full sibling duration
<!-- AC:END -->
