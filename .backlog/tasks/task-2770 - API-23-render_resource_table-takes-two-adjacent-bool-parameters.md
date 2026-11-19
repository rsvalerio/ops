---
id: TASK-2770
title: 'API-23: render_resource_table takes two adjacent bool parameters'
status: To Do
assignee: []
created_date: '2026-11-18 18:43'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2856'
modified_files:
  - extensions-terraform/plan/src/render.rs
  - extensions-terraform/plan/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'API-23:extensions-terraform/plan/src/render.rs:render_resource_table'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/render.rs:118` (`render_resource_table(changes, is_tty: bool, use_color: bool)`), call site `extensions-terraform/plan/src/lib.rs:222`

**What**: The public `render_resource_table` takes two adjacent `bool`s. The crate's own docs stress that they are deliberately separate knobs (width probing versus colour), and swapping them changes output in exactly the environment-dependent way the docs warn about. The call `render_resource_table(&classified, is_tty, use_color)` compiles just as well with the arguments swapped. The public `render_summary_table`, `render_outputs_table` and `run_plan_pipeline_to_with_tty` carry the same unnamed `bool`s (`use_color` / `is_tty`).

**Why it matters**: API-23 says any public signature with two or more `bool`s should take two-variant enums (for example `Width::Probe | Width::Fixed` and `Color::On | Color::Off`). Then a swap is a compile error, the call site reads clearly, and a later third state ("auto") has room to be added.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 render_resource_table's TTY and colour knobs are distinct enum types (or one options struct), so swapping them does not compile
- [ ] #2 render_summary_table, render_outputs_table and run_plan_pipeline_to_with_tty use the same types for their colour / TTY parameter
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.21 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
