---
id: TASK-1428
title: >-
  PERF-3: format_error_tail_with_stats allocates VecDeque per call for typical
  n<=20
status: Done
assignee:
  - TASK-1458
created_date: '2026-05-13 18:23'
updated_date: '2026-05-14 08:25'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/output.rs:110`

**What**: The error-tail formatter allocates `VecDeque::with_capacity(n)` plus a final `String::with_capacity(...)`. `n` is config-bounded (small, typically 5).

**Why it matters**: Error rendering is on the failure hot path; a stack-backed ring (smallvec or `[(usize, usize); N]` with manual wrap) skips both heap allocations for the dominant small-n case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace VecDeque with a stack-backed structure for typical small n
- [ ] #2 Bench shows no regression at the largest config-allowed n
<!-- AC:END -->
