---
id: TASK-0329
title: >-
  TEST-11: orphan-bar finalization test asserts only is_finished(), not Skipped
  status or rendered message
status: Done
assignee:
  - TASK-0414
created_date: '2026-04-26 09:19'
updated_date: '2026-04-26 10:20'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/tests.rs:559-595`

**What**: The new `run_finished_finalizes_orphan_running_bars` test only asserts `display.bars[i].is_finished()` for the orphaned bar. It does not assert:
- The bar's final message reflects `StepStatus::Skipped` (e.g. contains the skipped glyph / styled label produced by `render_and_wrap_step`).
- The orphan-finalization path uses the elapsed time from the bar (vs zero, or some other value).
- That `finish_bar` was actually called via `finalize_orphan_bars` rather than some unrelated code path that also flips `is_finished()` to true.

A regression that finalizes the bar with `StepStatus::Failed` (or with an empty/blank message) would silently pass this test. The fix's intent — that the row stays visible *with the right status* in the boxed frame — is not directly verified.

**Why it matters**: TEST-11 calls for assertions that exercise the behavior, not just liveness. `is_finished()` is a coarse boolean; the user-visible bug being prevented is "a hole in the boxed frame and a row count that disagrees with Done N/M" — neither of which is asserted. A reviewer reading this test cannot tell from the assertions alone what the orphan-finalization renders.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extend run_finished_finalizes_orphan_running_bars (or add a sibling test) to capture the orphan bar's final rendered message and assert it contains the Skipped status marker produced by render_and_wrap_step
- [ ] #2 Assert the elapsed value embedded in the rendered orphan-bar message is non-negative and derived from the bar's elapsed (not hardcoded zero)
- [ ] #3 Keep at least one assertion that distinguishes finalize_orphan_bars's path from any other code path that could mark the bar finished, e.g. by checking that the message reflects StepStatus::Skipped specifically (not Failed/Finished)
<!-- AC:END -->
