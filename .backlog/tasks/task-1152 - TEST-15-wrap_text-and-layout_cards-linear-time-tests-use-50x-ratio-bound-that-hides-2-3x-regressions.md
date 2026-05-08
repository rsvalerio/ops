---
id: TASK-1152
title: >-
  TEST-15: wrap_text and layout_cards linear-time tests use 50x ratio bound that
  hides 2-3x regressions
status: To Do
assignee:
  - TASK-1266
created_date: '2026-05-08 07:43'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - TEST
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/text_util.rs:303-332`

**What**: `wrap_text_handles_very_long_input_in_linear_time` and the sister `layout_cards_handles_large_workspace` (cards.rs:373-405) both assert ratio < 50.0 for a 10x input change. A genuine quadratic regression (~100x) is caught; a real-world 2-3x regression from extra clone-per-cell or N-log-N drift sits within the 50x band and slips through.

**Why it matters**: Threshold is loose enough that PERF-3-class regressions (TASK-0722 was specifically per-cell-clone) would pass under typical noise. The min-of-3 timing already de-noises enough for a tighter bound.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Tighten the ratio bound to a value reflecting actual measured spread on local hardware (10-15x for true linear)
- [ ] #2 Or replace the wall-clock assertion with an instrument-based check counting clones / String allocations
<!-- AC:END -->
