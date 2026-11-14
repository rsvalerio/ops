---
id: TASK-2152
title: 'API-14: cargo-update''s public constants, enum variants and struct fields carry no doc summaries'
status: Done
assignee: []
created_date: '2026-09-08 07:03'
updated_date: '2026-09-10 16:26'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions-rust/cargo-update/src/lib.rs
priority: low
ordinal: 65000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs`

**What**: Public items with no doc comment at all:

- `pub const NAME` (:26), `pub const DESCRIPTION` (:27), `pub const SHORTNAME` (:28), `pub const DATA_PROVIDER_NAME` (:29) — the crate's entire registration surface, four bare `pub const`s in a row.
- `UpdateAction::Update` (:36), `UpdateAction::Add` (:43), `UpdateAction::Remove` (:44) — undocumented, while the sibling `Downgrade` (:37-42) carries a five-line rationale comment, so rustdoc renders one variant with a paragraph and three with nothing.
- `UpdateEntry::action` (:51) and `UpdateEntry::name` (:52) — undocumented, while `from` (:53) and `to` (:54) are documented.
- `CargoUpdateResult::entries` (:64), `update_count` (:65), `add_count` (:71), `remove_count` (:72) — undocumented, while `downgrade_count` (:66-70) is.
- `CargoUpdateExtension` (:641-643) — its only doc line is `API-9 / TASK-0922: construct via the registered extension factory only.`, a maintenance note rather than a summary of what the type is.

**Why it matters**: These are `pub` on a workspace crate, so they render in `cargo doc` and are what a reader of the crate sees first. The half-documented pattern is worse than uniformly undocumented: a reader who sees a paragraph on `downgrade_count` and nothing on `add_count` reasonably infers the undocumented fields are the trivial ones, when in fact the difference is only which fields happened to be touched by a past fix.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every public const, enum variant and struct field in the crate has a one-line doc summary saying what it is
- [x] #2 CargoUpdateExtension's doc opens with a summary of the type rather than a construction note
- [x] #3 cargo doc for the crate shows no public item rendered with an empty description

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
UpdateEntry::action/name and the entry variants were already documented on landing by the TASK-2151 reshape; the remaining gaps (const quartet, UpdateAction variants, CargoUpdateResult fields, CargoUpdateExtension summary) documented here.
<!-- SECTION:NOTES:END -->
