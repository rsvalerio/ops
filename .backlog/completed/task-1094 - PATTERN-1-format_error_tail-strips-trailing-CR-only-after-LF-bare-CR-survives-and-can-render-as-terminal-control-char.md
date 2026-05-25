---
id: TASK-1094
title: >-
  PATTERN-1: format_error_tail strips trailing CR only after LF; bare CR
  survives and can render as terminal control char
status: Done
assignee: []
created_date: '2026-05-07 21:32'
updated_date: '2026-05-08 06:24'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/output.rs:46-52` (and inner stripping at line 66)

**What**: When stderr ends in a bare `\r` (no `\n`), the `if stderr.last() == Some(&b'\n')` branch is not entered, so the bare CR is preserved. The inner stripping at line 66 uses `wrapping_sub` to dodge underflow, but a corrupt input of just `[0x0D]` returns a non-empty result that round-trips via `String::from_utf8_lossy(b"\r")` rendering as a literal CR in error UIs — controlling cursor in some terminals.

**Why it matters**: Bare CR in error output can move the cursor to column 0 in operator terminals, masking subsequent log lines.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Bare CR in any line position is stripped, not just after a line-terminating LF
- [x] #2 A regression test passes b"\rfoo" and asserts the rendered tail does not contain a raw \r
- [x] #3 Document the CR/LF normalisation contract in the function rustdoc
<!-- AC:END -->
