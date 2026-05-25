---
id: TASK-1603
title: >-
  PATTERN-1: coverage_summary_view_sql hardcodes ROUND(..., 2) for percentages,
  creating per-file vs aggregate precision asymmetry
status: Done
assignee:
  - TASK-1635
created_date: '2026-05-21 22:53'
updated_date: '2026-05-22 10:13'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/views.rs:21-40`

**What**: `ROUND(..., 2)` is applied inside the view. Downstream consumers cannot un-round (they could recompute from underlying SUMs, but only if they know to). Per-row percentages stored in `coverage_files` (from llvm-cov) are full-precision floats; the view introduces a precision asymmetry between per-file and aggregate. Tests at `tests.rs:268-282` assert `abs < 0.01`, masking it.

**Why it matters**: Two coverage providers feeding the same dashboard will round inconsistently. `ROUND` in DuckDB returns DOUBLE — no storage saved, just lost precision.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Drop the ROUND(..., 2) from the view; let presentation-layer code format the percentage — or document why the view (not the consumer) is the right rounding boundary
- [ ] #2 If kept, externalize the precision as a const so future changes are deliberate
- [ ] #3 Add a test asserting 100.0 * SUM(covered) / SUM(count) reproduces the view's value within full f64 precision
<!-- AC:END -->
