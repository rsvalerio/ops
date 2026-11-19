---
id: TASK-2750
title: 'API-14: Public fields of ConfigFile, Written and Drift have no doc comments'
status: To Do
assignee: []
created_date: '2026-11-18 18:40'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2852'
modified_files:
  - extensions-rust/foundation/src/lib.rs
  - extensions-rust/foundation/src/compare.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions-rust/foundation/src/lib.rs:ConfigFile'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/foundation/src/lib.rs:34`, `extensions-rust/foundation/src/lib.rs:35`, `extensions-rust/foundation/src/lib.rs:74`, `extensions-rust/foundation/src/lib.rs:75`, `extensions-rust/foundation/src/compare.rs:25`

**What**: The crate's public types document themselves at the type level, but several public fields carry no doc summary: `ConfigFile::path` and `ConfigFile::template`, `Written::target` and `Written::outcome`, and `Drift::message` (its sibling `Drift::location` is documented). `Report`'s fields, by contrast, are all documented.

**Why it matters**: `Drift::message` and `Written::target` have a format the CLI renders verbatim (`target` gains a `:<table>` suffix for manifest edits); readers of the public API must go to the source to learn it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every public field of ConfigFile, Written and Drift has a one-line doc summary
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.16 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
