---
id: TASK-0216
title: 'TEST-1: emit_line_handles_empty_string performs no assertion'
status: Done
assignee: []
created_date: '2026-04-23 06:32'
updated_date: '2026-04-23 15:20'
labels:
  - rust-code-review
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/tests.rs:172`

**What**: Test calls display.emit_line("") and returns; only side-effect is stderr write which is neither captured nor asserted.

**Why it matters**: Assertion-free tests provide coverage illusion without catching regressions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Capture stderr via writer injection or assert on internal state (e.g., bars length unchanged)
- [ ] #2 Rename test to make the no-panic guarantee explicit
<!-- AC:END -->
