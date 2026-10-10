---
id: TASK-2584
title: 'DUP-1: Deduplicate the two byte-identical SQL identifier validators'
status: To Do
assignee: []
created_date: '2026-10-10 15:46'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - dup
dependencies: []
parent_task_id: 'TASK-2625'
modified_files:
  - extensions/sqlite/src/sql/ingest/sql.rs
  - extensions/sqlite/src/sql/validation.rs
priority: medium
ordinal: 1000
dedup_key: 'DUP-1:extensions/sqlite/src/sql/ingest/sql.rs:is_valid_column_name_const'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/ingest/sql.rs:218-233` (duplicate of `extensions/sqlite/src/sql/validation.rs:129-146`)

**What**: `is_valid_column_name_const` in `sql/ingest/sql.rs:218-233` has a body byte-identical to `is_valid_identifier_const` in `sql/validation.rs:129-146` (same `s.len()` bounds checks, same ASCII alnum/`_`/`$` acceptance loop, same early-return structure) — only the name differs. Two const validators for the same SQL identifier grammar now live in sibling modules.

**Why it matters**: The two copies can drift: a future tightening of the identifier grammar (e.g. rejecting `$`, or length limits) applied to one validator leaves the other permissively accepting what the new rule forbids. `validation.rs` is the crate's designated home for shared SQL validation; the ingest path should reuse it rather than carry a private twin. Fix: make `is_valid_identifier_const` `pub(super)` (or `pub(crate)`) in `validation.rs` and have `sql/ingest/sql.rs` call it, deleting the local copy.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the byte-identical const identifier validator in extensions/sqlite/src/sql/ingest/sql.rs (is_valid_column_name_const, lines 218-233) is removed and its callers use the surviving one in extensions/sqlite/src/sql/validation.rs (is_valid_identifier_const, lines 129-146)
- [ ] #2 cargo check -p ops-sqlite and cargo test -p ops-sqlite pass after the change

<!-- AC:END -->
