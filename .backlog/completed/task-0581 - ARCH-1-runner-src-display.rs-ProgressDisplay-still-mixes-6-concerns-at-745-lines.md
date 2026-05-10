---
id: TASK-0581
title: >-
  ARCH-1: runner/src/display.rs ProgressDisplay still mixes 6 concerns at 745
  lines
status: Done
assignee:
  - TASK-0644
created_date: '2026-04-29 05:17'
updated_date: '2026-04-29 17:04'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:67`

**What**: TASK-0332 split ProgressDisplay but the surviving struct still owns: (1) MultiProgress and indicatif lifecycle, (2) tap-file lifecycle including reopen-on-failure, (3) header/footer/separator bar bookkeeping, (4) box-vs-flat layout decisions, (5) per-event dispatch, (6) orphan-bar finalization. At 745 lines and 14 fields, 7 fields exclusively for tap-file or header/footer lifecycle.

**Why it matters**: ARCH-1. Tap-file concern is wholly orthogonal to progress rendering; could move into a tap.rs submodule. A TapWriter newtype owning (tap_file, tap_path, tap_truncation) plus tap_line_for and the RunFinished marker logic would let ProgressDisplay shrink.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Tap-file logic extracted into sibling module display/tap.rs
- [x] #2 ProgressDisplay retains tap: Option<TapWriter> field; tap_line_for and on_run_finished tap branch delegate to it
- [x] #3 File length below 600 lines after split
<!-- AC:END -->
