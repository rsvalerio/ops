---
id: TASK-1232
title: >-
  PERF-3: cap_streamed always allocates via from_utf8_lossy().into_owned() even
  when input is valid UTF-8
status: To Do
assignee:
  - TASK-1263
created_date: '2026-05-08 12:58'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:239-240`

**What**: `cap_streamed` builds the captured stdout/stderr String via `String::from_utf8_lossy(&head).into_owned()`. When `head` is already valid UTF-8 (the common case), `from_utf8_lossy` returns Cow::Borrowed and `into_owned()` copies the bytes into a fresh allocation, even though `head: Vec<u8>` could be consumed in place by `String::from_utf8`.

**Why it matters**: Per-stream per-spawn allocation copies up to OPS_OUTPUT_BYTE_CAP (default 4 MiB) bytes on the parallel hot path. Fast path could be `String::from_utf8(head).unwrap_or_else(|e| String::from_utf8_lossy(e.as_bytes()).into_owned())`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Try String::from_utf8 first and fall back to lossy on InvalidUtf8
- [ ] #2 Confirm parity for invalid bytes (U+FFFD substitution preserved)
- [ ] #3 Bench or comment pinning the saved alloc on the parallel path
<!-- AC:END -->
