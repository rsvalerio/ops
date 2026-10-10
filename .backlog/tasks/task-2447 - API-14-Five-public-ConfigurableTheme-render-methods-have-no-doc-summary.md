---
id: TASK-2447
title: 'API-14: Five public ConfigurableTheme render methods have no doc summary'
status: To Do
assignee: []
created_date: '2026-10-10 15:24'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2609'
modified_files:
  - crates/theme/src/configurable.rs
priority: medium
ordinal: 1000
dedup_key: 'API-14:crates/theme/src/configurable.rs:impl ConfigurableTheme'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:90` (`new`), `:168` (`render_plan_header`), `:194` (`render_summary_separator`), `:326` (`render`), `:443` (`render_summary`)

**What**: These five public methods on `ConfigurableTheme` — the crate's central public type — have no `///` doc comment at all. `render` and `render_summary` carry `//` comments above them, which do not render in rustdoc and do not satisfy the mandatory summary.

**Why it matters**: API-14 — the summary sentence is mandatory on every public item; it is what rustdoc lifts into the module index. `render` and `render_separator`'s shared budget/truncation contract (documented only at module level) is exactly the behaviour a caller needs summarized on the entry points.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 new, render_plan_header, render_summary_separator, render and render_summary each carry a /// summary of roughly 15 words followed by a blank line
- [ ] #2 The existing // comments on render and render_summary are either promoted to /// or retained as implementation notes below a doc summary
<!-- AC:END -->
