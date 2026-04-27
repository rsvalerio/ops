---
id: TASK-0333
title: >-
  ERR-1: finalize_orphan_bars skips completion accounting, leaving stale footer
  counts
status: Done
assignee:
  - TASK-0414
created_date: '2026-04-26 09:32'
updated_date: '2026-04-26 10:20'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:602-617`

**What**: `finalize_orphan_bars` marks each unfinished bar as Skipped and re-renders, but does not increment `self.completed_steps`. `on_run_finished` then emits a "Done N/M" footer where N still reflects only steps that received explicit terminal events.

**Why it matters**: Under fail_fast cancellation, the visible bar row is finalized but the footer "Done 1/3" disagrees with the 3 finished rows on screen — exactly the disagreement the function was added to prevent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Increment self.completed_steps for each finalized orphan bar (and self.failed_steps if treated as failure on fail_fast)
- [ ] #2 Add a test asserting that after RunFinished with one orphan bar, the footer message reflects total_steps in its completed count
<!-- AC:END -->
