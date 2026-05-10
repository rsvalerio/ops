---
id: TASK-1174
title: >-
  ERR-1: dropped-output count send dropped on closed outer channel without
  diagnostic
status: Done
assignee:
  - TASK-1268
created_date: '2026-05-08 08:08'
updated_date: '2026-05-09 17:37'
labels:
  - code-review-rust
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:618`

**What**: When `dropped_outputs > 0`, the code does `let _ = tx.send(RunnerEvent::StepOutputDropped { ... }).await`. The comment claims "Awaited send so the count itself can never be silently dropped", but a closed receiver returns `Err(SendError)` which is silently discarded — the user never sees the dropped-line count for that case.

**Why it matters**: If the display side has already dropped its receiver (e.g. shutdown race, fail_fast tear-down), the count is silently lost. The whole point of TASK-0457 is that this count never disappears.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tx.send(...) failure logs a tracing::warn! (or pushes to a fallback channel) so the count survives even when the outer receiver is closed.
- [x] #2 Regression test drops the receiver before the producer flushes its dropped-count event and asserts a diagnostic is recorded.
<!-- AC:END -->
