---
id: TASK-0771
title: >-
  CL-3: finalize_orphan_bars increments completed_steps for skipped/aborted
  rows, conflating completed with terminal
status: Triage
assignee: []
created_date: '2026-05-01 05:56'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/finalize.rs:40`

**What**: completed_steps counter is incremented by both finish_step (Succeeded/Failed/Skipped) and finalize_orphan_bars (orphan Skipped). Footer renders "Done N/M" using this counter, so a fail_fast run shows "Done 3/3 in 1.2s" even though only one step succeeded and two were aborted.

**Why it matters**: Label "Done" implies success; value implies completion-including-cancellation. Boxed-layout snapshot uses completed_steps with no failure annotation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either rename completed_steps → terminal_steps and document the semantics, or split into succeeded_steps / terminal_steps so summary rendering can distinguish them
- [ ] #2 Update boxed footer rendering to surface "Failed N/M" or "N succeeded, K skipped, M failed of T" rather than a single "Done" count
- [ ] #3 Regression test asserting the summary line under fail_fast shows the correct counts
<!-- AC:END -->
