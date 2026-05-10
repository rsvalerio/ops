---
id: TASK-0331
title: 'CONC-5: Tap file writes use std::fs::File from event-handling path'
status: Done
assignee:
  - TASK-0414
created_date: '2026-04-26 09:32'
updated_date: '2026-04-26 10:27'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:485-497`

**What**: `tap_line` writes via `writeln!` to a `std::fs::File`. Tap file is opened with `std::fs::File::create` (line 108) and written via blocking writeln on every StepOutput event.

**Why it matters**: If the handle_event consumer is ever moved into an async task, the blocking write per stderr line stalls the runtime under any moderately chatty command (cargo build emits thousands of lines).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either document an invariant that handle_event must never be polled from async, or switch the tap file to a buffered/async writer fed via an mpsc channel and a dedicated writer task
- [ ] #2 Test that draining a 10k-line StepOutput burst into a Display with tap enabled completes without blocking a concurrent tokio task
<!-- AC:END -->
