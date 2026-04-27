---
id: TASK-0337
title: 'READ-5: on_step_skipped discards real elapsed duration'
status: Done
assignee:
  - TASK-0414
created_date: '2026-04-26 09:33'
updated_date: '2026-04-26 10:20'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:545-547`

**What**: `on_step_skipped` hard-codes 0.0 for duration_secs when finalizing the bar, even though the bar's own elapsed timer is available (used in finalize_orphan_bars at line 608).

**Why it matters**: Skipped bars under fail_fast appear with "0.00s" elapsed in the rendered row, hiding how much CPU actually went to the cancelled task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Read self.bars[i].elapsed() before disabling the steady tick and pass it through to finish_step
- [ ] #2 Test asserts a step skipped after running >0ms reports a non-zero elapsed in the rendered line
<!-- AC:END -->
