---
id: TASK-0457
title: >-
  CONC-7: exec_standalone try_send silently drops non-terminal output events
  under backpressure
status: Done
assignee:
  - TASK-0537
created_date: '2026-04-28 05:45'
updated_date: '2026-04-28 17:06'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:362`

**What**: `exec_standalone` uses `local_tx.try_send(ev)` and on `TrySendError::Full` logs `tracing::debug!("...dropping event under backpressure")`, discarding the line. The 256-slot local + 32×256 outer channel sizing assumes steady-state, but a chatty cargo build can burst past 256 lines between display pumps; stdout/stderr lines are then silently lost from tap and error-detail tail.

**Why it matters**: Comment at exec.rs:342 acknowledges terminal events get backpressure precisely because dropping them was a bug — but stdout/stderr drops have the same observable consequence: operator only sees the final exit message and not the lines that explain it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Output events are forwarded with backpressure (e.g. blocking_send on a dedicated thread, or async callback), or an explicit StepOutputDropped { id, dropped_count } event is emitted so display can show "(N output lines dropped under load)"
- [x] #2 Regression test simulates a chatty producer (>1000 lines/burst) and asserts either zero drops or a dropped-count event is emitted with correct id
<!-- AC:END -->
