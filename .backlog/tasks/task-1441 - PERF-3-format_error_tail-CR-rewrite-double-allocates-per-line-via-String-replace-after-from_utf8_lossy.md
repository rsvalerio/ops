---
id: TASK-1441
title: >-
  PERF-3: format_error_tail CR-rewrite double-allocates per line via
  String::replace after from_utf8_lossy
status: Done
assignee:
  - TASK-1458
created_date: '2026-05-13 18:40'
updated_date: '2026-05-14 08:25'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/output.rs:147-152`

**What**: For every captured line that contains a `\r`, the code performs `String::from_utf8_lossy(&buf[s..e])` (alloc 1) followed by `.replace('\r', "\n")` (alloc 2) before pushing into the output buffer. The CR-rewrite happens per-line of the tail window. Distinct from TASK-1428 (VecDeque pre-alloc) and TASK-1422 (emit_to writeln-per-line).

**Why it matters**: `format_error_tail` runs on every failed step's stderr capture; CR-bearing tails (CRLF subprocess output on Windows, cargo-style progress bars on Unix) trip the double-alloc per line. Substituting in a single pass over the decoded chars eliminates one allocation per affected line.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Push the decoded bytes into the output buffer once, substituting \r -> \n inline (e.g. for c in decoded.chars()), with no intermediate String::replace allocation
- [ ] #2 Existing CR-rewrite tests still pass; new test confirms a buffer of N CR-only lines produces at most one extra allocation beyond the result String
<!-- AC:END -->
