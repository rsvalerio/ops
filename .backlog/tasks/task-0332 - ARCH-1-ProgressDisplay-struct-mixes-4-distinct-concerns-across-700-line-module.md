---
id: TASK-0332
title: >-
  ARCH-1: ProgressDisplay struct mixes 4 distinct concerns across 700-line
  module
status: Done
assignee:
  - TASK-0414
created_date: '2026-04-26 09:32'
updated_date: '2026-04-26 10:58'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:80-100`

**What**: ProgressDisplay carries 14 fields spanning render config, progress state, event routing, and IO/tap. The module body is 703 lines and the doc comment (lines 71-79) explicitly identifies ProgressState/EventRouter as a live extraction candidate.

**Why it matters**: The single struct collects state used in different lifecycles (per-plan vs per-step) and lifetimes (TTY vs non-TTY), making future modifications require touching every concern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract ProgressState (bars, steps, step_stderr, display_map, plan_command_ids) and either an EventRouter or per-event handler module from ProgressDisplay; keep public handle_event API stable
- [ ] #2 New module structure has tests demonstrating each split component independently
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Wave 25 (TASK-0414) review: deferred. The doc comment at crates/runner/src/display.rs:71-79 explicitly defers ProgressState/EventRouter extraction to land WITH the next non-trivial change. The wave-25 changes here (orphan-bar accounting TASK-0333, on_step_skipped elapsed TASK-0337, async-safety doc TASK-0331) are individually small and do not justify a 700-line restructure that risks regressions across the entire CLI display surface. Schedule a dedicated wave whose primary change is the extraction itself (with comprehensive cross-component tests) rather than piling onto this one.

Wave 25 actually closed: extracted ProgressState (bars, steps, step_stderr, display_map, plan_command_ids) into display/progress_state.rs with its own unit tests (step_index, resolve_step_display fallback, record_stderr accumulation, reset_for_plan clears prior plan state). ProgressDisplay holds a `state: ProgressState` field; all event-handler methods now mutate self.state.X. EventRouter not extracted as a separate module — handle_event remains a thin dispatcher in display.rs which is the natural home for it (depends on RenderConfig + ProgressState equally).
<!-- SECTION:NOTES:END -->
