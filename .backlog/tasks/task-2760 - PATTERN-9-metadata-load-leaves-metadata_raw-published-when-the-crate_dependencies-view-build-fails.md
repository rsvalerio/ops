---
id: TASK-2760
title: 'PATTERN-9: metadata load leaves metadata_raw published when the crate_dependencies view build fails'
status: To Do
assignee: []
created_date: '2026-11-18 18:41'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
parent_task_id: 'TASK-2853'
modified_files:
  - extensions-rust/metadata/src/ingestor.rs
priority: medium
ordinal: 1000
dedup_key: 'PATTERN-9:extensions-rust/metadata/src/ingestor.rs:MetadataIngestor::load'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:94` (`MetadataIngestor::load` -> `build_views`, lines 197-204)

**What**: `load` tears the just-published `metadata_raw` table down through explicit `drop_metadata_tables` calls at individual exits: inside `validate_published` (line 134) and on the `upsert_data_source` failure (line 114). `build_views` (line 94) is not covered. It first runs `ops_sqlite::sql::load_json_string`, which drops, creates and inserts the single `metadata_raw` row with no transaction. Then it runs the `crate_dependencies` DROP VIEW + CREATE VIEW batch. If that batch fails (for example SQLITE_BUSY, a full disk or an I/O error), `?` returns while `metadata_raw` still holds its row. At that point there is no `crate_dependencies` view and no `data_sources` provenance row.

**Why it matters**: The orchestrator (`extensions/sqlite/src/sql/ingest/orchestrator.rs`, `provide_via_ingestor`) probes `table_has_data("metadata_raw")` and skips re-ingest when it is true. Every later run therefore keeps the half-built state until an explicit refresh: `crate_dependencies` consumers fail and `data_sources` certifies nothing. This is the sticky-failure mode that the rest of `load` guards against. Teardown is a cleanup call repeated at each exit rather than a guard, so this exit was missed. PATTERN-9 asks for a guard whose `Drop` tears the tables down unless it is disarmed after the upsert succeeds, or for the table and view build to run inside one transaction.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A failure of the crate_dependencies view batch after metadata_raw is populated leaves no populated metadata_raw table behind
- [ ] #2 Teardown on every post-publish failure in load goes through one mechanism (a disarmable guard or a single transaction), not per-exit calls
- [ ] #3 A test forces the view build to fail after the table load and asserts metadata_raw is absent or empty afterwards
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.18 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
