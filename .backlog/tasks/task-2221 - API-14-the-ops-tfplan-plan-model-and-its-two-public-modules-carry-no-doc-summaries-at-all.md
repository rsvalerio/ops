---
id: TASK-2221
title: 'API-14: the ops-tfplan plan model and its two public modules carry no doc summaries at all'
status: To Do
assignee: []
created_date: '2026-09-08 07:21'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions-terraform/plan/src/model.rs
  - extensions-terraform/plan/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/model.rs:1-26`, `extensions-terraform/plan/src/model.rs:118-126`, `extensions-terraform/plan/src/lib.rs:11-12`, `extensions-terraform/plan/src/lib.rs:74-77`

**What**: Undocumented public surface:

- `pub mod model;` and `pub mod render;` — no `//!` module docs, so both index pages are blank headers.
- `Plan`, `ResourceChange`, `Change` and every one of their fields (`format_version`, `resource_changes`, `output_changes`, `address`, `module`, `mode`, `r#type`, `name`, `change`) — no docs. These are the crate's deserialization contract with the terraform JSON plan format; nothing states which terraform `format_version`s are supported, or that `Option` here means "absent from the document" rather than "not applicable".
- `ClassifiedChange` and all six fields — no docs. `mode` in particular is populated (defaulting to `"managed"`) and sanitized but never rendered by this crate, which only a reader of `classify_plan`'s inline comment would know.
- `has_changes` — no doc comment whatsoever.
- `Action::classify` is `pub`, returns a value with no side effect, and has no `#[must_use]`; `Action::color`, `is_change`, `label` and `sort_priority` carry bare `#[must_use]` with no message.

`Action` itself is the counter-example that shows the gap: its `Unknown` variant is well documented, and the contrast makes the surrounding blanks look like oversight rather than a deliberately minimal surface.

**Why it matters**: All of these types are `#[non_exhaustive]` (TASK-0832) precisely because they are a supported public surface that outside code is expected to match on and construct against. A supported surface with no summaries pushes every consumer into reading `classify_plan` to learn what `mode` and `module` actually contain, and leaves the `Option`-means-absent convention as folklore.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both public modules have //! summaries, and Plan, ResourceChange, Change and ClassifiedChange plus their fields have /// summaries
- [ ] #2 The model docs state which terraform plan format_version values are supported and that Option fields mean 'absent from the document'
- [ ] #3 has_changes is documented; Action::classify gains #[must_use] and the existing bare #[must_use] attributes on Action carry a reason
<!-- AC:END -->
