---
id: TASK-2497
title: 'READ-13: lib.rs crate docs narrate TASK history and prior-implementation shapes'
status: To Do
assignee: []
created_date: '2026-10-10 15:31'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - read
dependencies: []
parent_task_id: 'TASK-2617'
modified_files:
  - extensions-rust/test-coverage/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/test-coverage/src/lib.rs:crate root'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:4-14,16-19,28-33,42-46,65,87-95,102-104`

**What**: The crate-root docs are process artifacts, not API documentation. The module doc narrates the TASK-1559 monolith split ("the previous monolithic `lib.rs` (412 lines) mixed six concerns. The crate is now split into: ..."), the `cfg_attr(test, allow(...))` block opens with "READ-10 / TASK-1946: ... the three cast lints that used to sit alongside it were a copied template", the module list comment opens with "API-14 / TASK-2198", the API-9 / TASK-0922 and TASK-1601/1602 comments narrate which callers used to exist, and `load_coverage`'s doc tells the story of the previous `()`-returning signature (READ-5 / TASK-0808), the `#[must_use]` addition (API-5 / TASK-1561), and the IngestDir anchor change (SEC-25 / TASK-2054).

**Why it matters**: READ-13: docs must describe the end state, not the journey that produced it. Rule/TASK provenance tags and prior-implementation narration are meaningless to a reader using the API, go stale on the next change while looking authoritative, and duplicate information that belongs in git history/ADRs. Sibling crates (theme, about, sqlite) have already had these stripped in dedicated waves; this crate was missed.

**Note**: `ingest.rs` is the only file in the crate already clean of TASK tags.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Module and item docs in lib.rs describe the current structure and contract with no TASK-XXXX references and no narration of prior shapes (no 'the previous monolithic', 'used to', 'no longer' history)
<!-- AC:END -->
