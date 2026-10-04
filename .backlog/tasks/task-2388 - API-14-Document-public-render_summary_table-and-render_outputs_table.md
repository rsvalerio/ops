---
id: TASK-2388
title: 'API-14: Document public render_summary_table and render_outputs_table'
status: To Do
assignee: []
created_date: '2026-10-04 14:15'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2424'
modified_files:
  - extensions-terraform/plan/src/render.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions-terraform/plan/src/render.rs:render_summary_table'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/render.rs:54` (`render_summary_table`), `extensions-terraform/plan/src/render.rs:~214` (`render_outputs_table`)

**What**: Two public functions in the public `render` module carry only `#[must_use]` and no `///` doc summary. Their siblings (`render_resource_table`) are documented. Neither explains its inputs (`use_color`), its output shape (summary table plus the `Plan: N to add...` line, or the "No changes" string; the outputs table plus unknown banner), or the empty-input behaviour (summary returns a "No changes" message, outputs returns an empty string).

**Why it matters**: These are the crate's public rendering API, called by library consumers, and API-14 requires a doc summary on every public item. The empty-input asymmetry (non-empty string vs empty string) is a caller-visible behaviour that is currently undocumented.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 render_summary_table and render_outputs_table each have a short doc summary naming what they render and the empty-input return value
- [ ] #2 cargo clippy with missing_docs on the crate reports no undocumented public item in render.rs
<!-- AC:END -->
