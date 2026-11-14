---
id: TASK-2384
title: 'DUP-3: Repeated read_dir-with-warn and per-entry-warn blocks in ops-about-terraform'
status: Done
assignee: []
created_date: '2026-10-04 14:13'
updated_date: '2026-10-04 16:14'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2424'
modified_files:
  - extensions-terraform/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-3:extensions-terraform/about/src/libead_dir-warn-blocks'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/about/src/lib.rs:154-165, 166-177, 1077-1088, 1089-1100, 1146-1157, 1158-1169`

**What**: Two near-identical blocks are each repeated three times:
1. `match std::fs::read_dir(p) { Ok(e) => e, Err(NotFound) => <empty>, Err(e) => { tracing::warn!(..); <empty> } }` in `fallback_tf_paths`, `count_local_modules` and `contains_terraform_source`.
2. `.filter_map(|res| match res { Ok(entry) => Some(entry), Err(e) => { tracing::warn!(..); None } })` in the same three functions (only the field name and message text differ).
The identical unterminated-string `tracing::warn!` is also emitted from two arms of `extract_required_version` (lines 244-249 and 283-295).

**Why it matters**: Three copies of the same IO policy must change together (for example if the log fields or NotFound handling change); drift between them would make the module-level "Manifest IO policy" untrue for one path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A single helper (for example read_dir_logged(dir, what) -> Option<impl Iterator<Item = DirEntry>>) owns the NotFound-silent / other-error-warn policy and the per-entry warn, and the three call sites use it
- [x] #2 Existing warn-path tests (versions.tf is a directory, modules is a file) still pass and log the same fields

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
read_dir_logged(dir, what) owns the policy. The three sites now log the directory under one field name, dir (previously root / modules_dir / module_dir), with what naming the role in the message; error field and the asserted message text are unchanged and the existing warn-path tests pass unmodified.
<!-- SECTION:NOTES:END -->
