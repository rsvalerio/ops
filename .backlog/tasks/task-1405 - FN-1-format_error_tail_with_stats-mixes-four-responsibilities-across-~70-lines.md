---
id: TASK-1405
title: >-
  FN-1: format_error_tail_with_stats mixes four responsibilities across ~70
  lines
status: Done
assignee:
  - TASK-1458
created_date: '2026-05-13 18:10'
updated_date: '2026-05-14 08:25'
labels:
  - code-review-rust
  - FN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/output.rs:85-155`

**What**: `format_error_tail_with_stats` interleaves trailing-terminator trimming, backwards line-range walking, CR-normalisation, and UTF-8 decoding in one function spanning ~70 lines.

**Why it matters**: Four distinct concerns and three sets of bytewise edge cases share one body; extracting `trim_trailing_terminator`, `collect_tail_ranges`, and `decode_with_cr_normalisation` would let each be tested in isolation and would make the byte/line invariants legible at a glance.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 format_error_tail_with_stats decomposes into helpers each under 30 lines
- [ ] #2 Existing tail-formatting tests continue to pass without modification
<!-- AC:END -->
