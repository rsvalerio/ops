---
id: TASK-2077
title: 'DUP-1: unchecked_items duplicated verbatim in create.rs and edit.rs'
status: Done
assignee: []
created_date: '2026-09-07 22:57'
updated_date: '2026-09-09 18:44'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2242'
modified_files:
  - crates/backlog/src/cmd/create.rs
  - crates/backlog/src/cmd/edit.rs
priority: medium
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/create.rs:31`, `crates/backlog/src/cmd/edit.rs:145`

**What**: The helper `fn unchecked_items(texts: &[String]) -> Vec<AcItem>` exists twice with byte-identical bodies (9 lines each): once in `create.rs` (for `--ac`/`--dod` at create time) and once in `edit.rs` (for wholesale section replacement). Both build fresh unchecked `AcItem`s from their texts.

**Why it matters**: Two copies of the same construction rule for the crate's most-edited shape (checkbox items) will drift: a future change (e.g. trimming, validation, a checked-state policy for replacements) applied to one leaves the other behind, and create/edit then disagree on the same on-disk section. `model.rs` already owns `AcItem` — the constructor belongs there.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A single shared constructor exists in crates/backlog/src/model.rs (e.g. AcItem::unchecked_all or an equivalent), used by both run_create and run_edit
- [x] #2 No body-duplicate of the helper remains under crates/backlog/src/cmd/

<!-- AC:END -->
