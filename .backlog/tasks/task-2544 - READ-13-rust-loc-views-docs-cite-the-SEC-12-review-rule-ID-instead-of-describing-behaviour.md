---
id: TASK-2544
title: 'READ-13: rust-loc views docs cite the SEC-12 review rule ID instead of describing behaviour'
status: To Do
assignee: []
created_date: '2026-10-10 15:38'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - readability
dependencies: []
parent_task_id: 'TASK-2626'
modified_files:
  - extensions-rust/loc/src/views.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/loc/src/views.rs:views'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/views.rs:14` (doc on public const `RUST_LOC_FILES_LOAD`), `extensions-rust/loc/src/views.rs:67` (doc on test fn `rust_loc_files_load_declares_typed_columns`)

**What**: Both doc comments open with `SEC-12: ...` — a review-rule citation framing otherwise-good explanations of the const-validated identifiers and the bound-`?1` staging. TASK-2192 (Done) removed exactly this framing from the crate and its AC #1 states "no doc comment or section heading in the crate contains a TASK-#### reference or a review rule ID"; these two sites reintroduce it, and the views.rs one renders into `cargo doc` output for a public const.

**Why it matters**: Rule IDs only mean something to a reader with the backlog open, they decay as rules are renumbered, and they narrate the review process that produced the code rather than the end state. The backlog task is the audit trail; the doc comment is the place for the current behaviour. Same shape as the regression filed in TASK-2369/TASK-2499 for other crates.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 No doc comment in the crate opens with or cites a review rule ID (SEC-12, ERR-n, TASK-nnnn); the identifier-validation and bound-parameter explanations are preserved, rewritten as descriptions of current behaviour
- [ ] #2 cargo doc -p ops-rust-loc builds cleanly and the RUST_LOC_FILES_LOAD doc reads as a description of the load spec
<!-- AC:END -->
