---
id: TASK-0164
title: >-
  DUP-3: ConfigOverlay->base merge blocks repeat the nested if-let Some(overlay)
  / if-let Some(field) pattern 4x
status: Done
assignee: []
created_date: '2026-04-22 21:24'
updated_date: '2026-04-23 14:32'
labels:
  - rust-code-review
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/core/src/config/merge.rs:52-70

**What**: merge_config hand-writes the same 3-line pattern four times for data.path, extensions.enabled, about.fields, and stack. Each block does: if let Some(x_overlay) = x { if let Some(field) = &x_overlay.field { base.foo = Some(field.clone()); } }. Adding another Option-in-Option overlay field means copy-pasting the block again. A small helper (fn copy_some<T: Clone>(dst: &mut Option<T>, src: Option<&Option<T>>)) or a consistent use of the existing merge_field on Option<T> collapses all four blocks into one-liners and makes the intent uniform with merge_output / merge_indexmap helpers at the top of the file.

**Why it matters**: DUP-3 / DUP-5. Reduces maintenance risk of forgetting a field or getting the pattern wrong on the 5th add.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the four inline if-let blocks with a shared helper (or extend merge_field to handle Option<Option<T>>)
- [ ] #2 Verify behavior: only explicitly-set Some values overwrite; None preserves base
<!-- AC:END -->
