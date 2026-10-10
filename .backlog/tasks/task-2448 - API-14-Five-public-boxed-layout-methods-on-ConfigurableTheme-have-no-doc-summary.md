---
id: TASK-2448
title: 'API-14: Five public boxed-layout methods on ConfigurableTheme have no doc summary'
status: Done
assignee: []
created_date: '2026-10-10 15:24'
updated_date: '2026-10-10 21:21'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2609'
modified_files:
  - crates/theme/src/configurable/boxed.rs
priority: medium
ordinal: 1000
dedup_key: 'API-14:crates/theme/src/configurable/boxed.rs:impl ConfigurableTheme'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable/boxed.rs:47` (`render_error_detail`), `:91` (`step_column_reserve`), `:99` (`box_top_border`), `:121` (`box_bottom_border`), `:155` (`wrap_step_line`)

**What**: All five public methods in the boxed-layout impl block lack `///` doc comments. They are public API — reachable through the re-exported `ConfigurableTheme` type even though the `configurable` module itself is private.

**Why it matters**: API-14 — the summary sentence is mandatory on every public item. These methods carry real contracts a caller cannot guess (frame geometry, `columns == 0` meaning "no budget", `Option` return gated on `LayoutKind::Boxed`), currently documented only in scattered inline comments or not at all.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 render_error_detail, step_column_reserve, box_top_border, box_bottom_border and wrap_step_line each carry a /// doc summary
- [x] #2 Each summary states the LayoutKind gating and the columns==0 'no budget' convention where applicable

<!-- AC:END -->
