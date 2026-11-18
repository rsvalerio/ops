---
id: TASK-2829
title: 'PATTERN-9: load_with_sidecar leaves a populated table without its view or provenance row on a mid-load failure, and the orchestrator never retries it'
status: Done
assignee: []
created_date: '2026-11-18 18:50'
updated_date: '2026-11-18 21:47'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
parent_task_id: 'TASK-2853'
modified_files:
  - extensions/sqlite/src/ingestor.rs
  - extensions/sqlite/src/sql/ingest/sql.rs
  - extensions/sqlite/src/sql/ingest/orchestrator.rs
priority: medium
ordinal: 1000
dedup_key: 'PATTERN-9:extensions/sqlite/src/ingestor.rs:SidecarIngestorConfig::load_with_sidecar'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/ingestor.rs:173-201` (`SidecarIngestorConfig::load_with_sidecar`, steps 3-6), `extensions/sqlite/src/sql/ingest/sql.rs:393-404` (`load_json_string`), `extensions/sqlite/src/sql/ingest/orchestrator.rs:123-148` (`provide_via_ingestor`)

**What**: `load_with_sidecar` runs the table load (`load_json_string`: `DROP TABLE; CREATE TABLE` batch, then the `INSERT … FROM json_each(?1)`), then the view batch (`DROP VIEW; CREATE VIEW`), then `COUNT(*)`, then `persist_record` (checksum + `upsert_data_source`) as separate autocommit statements with no transaction. If anything after the INSERT fails (the view batch, the count, the checksum read, or the upsert, e.g. on SQLITE_BUSY after the 5 s busy timeout, a full disk or an I/O error), `?` returns with the data table already populated. The view and the `data_sources` row are missing.

`provide_via_ingestor` decides whether to collect and load only via `table_has_data(table_name)`, which is now true. Every later run skips collect and load and goes straight to `query_fn`, and the half-built state persists until the user passes an explicit refresh. Consumers of the missing view degrade silently: `query_rust_loc_summary` returns an empty vec when `rust_loc_summary` is absent. The doc comment's "idempotent on retry … just call `load_with_sidecar` again" never happens in practice, because the orchestrator never calls it again.

This is the same sticky-failure mode TASK-2760 reports for `MetadataIngestor::load` in ops-metadata. Here it applies to every sidecar ingestor (tokei, rust-loc, test-coverage) through the shared helper.

**Why it matters**: A single transient failure permanently produces wrong or empty about/loc/coverage output with no error on later runs. Teardown is not guaranteed on the failure path: there is neither a guard nor a transaction.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The table load, view batch, count and data_sources upsert in load_with_sidecar run inside one transaction (or a disarmable guard drops the table on any failure after the INSERT)
- [x] #2 A failure after the table is populated leaves no populated data table behind, so the next provide_via_ingestor call re-collects and re-loads
- [x] #3 A test forces the view batch (or the upsert) to fail after the insert and asserts that a subsequent provide_via_ingestor call re-runs collect and load

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.30 (code-review-rust run, 2026-11-18).

Fixed in TASK-2853: load_with_sidecar runs table load, view batch, count and upsert_data_source_with in one transaction; checksum moved before the lock. Test: orchestrator failed_sidecar_load_is_retried_by_the_next_call.

<!-- SECTION:NOTES:END -->
