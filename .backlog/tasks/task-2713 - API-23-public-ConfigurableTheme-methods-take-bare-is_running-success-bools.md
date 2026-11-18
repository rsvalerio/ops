---
id: TASK-2713
title: 'API-23: public ConfigurableTheme methods take bare is_running/success bools'
status: To Do
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - api-design
dependencies: []
parent_task_id: 'TASK-2845'
modified_files:
  - crates/theme/src/configurable.rs
  - crates/theme/src/step_line_theme.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.06-5'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs` — `ConfigurableTheme::step_prefix_parts(status, is_running: bool)` (:253), `icon_prefix_parts(icon, is_running: bool)` (:263), `render_prefix(step, is_running: bool)` (:277), `render_separator(prefix, duration_str, columns, is_running: bool)` (:290), `render_summary(success: bool, elapsed_secs)` (:454); `SlotLine::is_running` / `BoxSnapshot::success` fields in `crates/theme/src/step_line_theme.rs` follow the same shape.

**What**: Each public method takes a positional `bool` that its name does not explain, so call sites read `theme.render_prefix(&step, false)` or `theme.render_summary(true, 1.2)`. `render_separator` also mixes it with a `usize` column budget in a four-argument signature.

**Why it matters**: The meaning is invisible at the call site, a reviewer has to open the signature, and the running/finished distinction is already a third state away from needing an enum (e.g. queued rows). A two-variant enum (`RowState::Running` / `RowState::Finished`, `Outcome::Success` / `Outcome::Failure`) reads at the call site and cannot be passed the wrong way round.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The listed public methods take a named enum (or derive the state from an argument already passed, e.g. StepLine::status) instead of a bare bool
- [ ] #2 Call sites across the workspace are updated and read without consulting the signature
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.06 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
