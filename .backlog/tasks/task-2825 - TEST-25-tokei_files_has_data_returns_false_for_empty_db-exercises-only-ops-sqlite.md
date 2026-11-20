---
id: TASK-2825
title: 'TEST-25: tokei_files_has_data_returns_false_for_empty_db exercises only ops-sqlite'
status: Done
assignee: []
created_date: '2026-11-18 18:49'
updated_date: '2026-11-18 22:52'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2855'
modified_files:
  - extensions/tokei/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-25:extensions/tokei/src/tests.rs:tokei_files_has_data_returns_false_for_empty_db'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/tests.rs:556` (`tokei_files_has_data_returns_false_for_empty_db`)

**What**: The test opens an in-memory DB, runs `ops_sqlite::init_schema`, and asserts that `ops_sqlite::sql::table_has_data(&db, "tokei_files")` is false. No ops-tokei code runs. `init_schema` does not create `tokei_files` (that table comes from `TokeiIngestor::load`), so the test only verifies ops-sqlite's handling of a table that does not exist. The string `"tokei_files"` is its only link to this crate.

**Why it matters**: It is a test of a sibling crate placed in the wrong suite. It gives no coverage of tokei logic, and it would not catch a regression in table_has_data for tables that do exist. That behaviour belongs to ops-sqlite's own tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The test is removed from ops-tokei, or changed to exercise tokei code (e.g. the provider's provide_from_db path on an empty DB triggering ingestion)
- [x] #2 If table_has_data on a missing table is not already covered in ops-sqlite, an equivalent test exists there

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.32 (code-review-rust run, 2026-11-18).

AC2: table_has_data on a missing table is already covered by ops-sqlite's table_has_data_no_table (extensions/sqlite/src/sql/ingest/sql.rs).

<!-- SECTION:NOTES:END -->
