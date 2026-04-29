---
id: TASK-0582
title: 'FN-1: ProgressDisplay::on_run_finished mixes 4 unrelated finishing concerns'
status: Done
assignee:
  - TASK-0644
created_date: '2026-04-29 05:17'
updated_date: '2026-04-29 17:04'
labels:
  - code-review-rust
  - FN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:641`

**What**: on_run_finished (lines 641-698) dispatches: (1) finalize orphan bars, (2) emit tap-truncation warning and re-open tap to append marker (with a second silently-swallowed open failure), (3) finalize header/footer for boxed layout, (4) format and emit flat-summary fallback. ~57 lines, three early returns, three independent branches.

**Why it matters**: FN-1: each concern has its own state machine and failure mode. Lifting the tap-truncation block makes the second-open fallback explicit and testable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Tap-truncation block extracted into report_tap_truncation(&mut self)
- [x] #2 Boxed-finalization block extracted into finalize_boxed_layout
- [x] #3 Flat-summary fallback extracted into finalize_flat_layout
- [x] #4 on_run_finished becomes a 5-10-line dispatcher
<!-- AC:END -->
