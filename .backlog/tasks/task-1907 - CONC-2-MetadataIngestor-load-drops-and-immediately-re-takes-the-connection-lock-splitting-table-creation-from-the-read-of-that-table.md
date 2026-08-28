---
id: TASK-1907
title: >-
  CONC-2: MetadataIngestor::load drops and immediately re-takes the connection
  lock, splitting table creation from the read of that table
status: To Do
assignee:
  - TASK-1999
created_date: '2026-08-27 15:39'
updated_date: '2026-08-28 14:14'
labels:
  - code-review-rust
  - idioms-correctness
dependencies: []
modified_files:
  - extensions-rust/metadata/src/ingestor.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/ingestor.rs:46-59`

**What**:

```rust
{
    let conn = db.lock()?;
    build_views(&conn, &path)?;
}                                  // guard dropped here…
let conn = db.lock()?;             // …and immediately re-acquired
let record_count = query_record_count(&conn)?;
...
let workspace_root = extract_workspace_root(&conn)?;
drop(conn);
```

The scoping block ends one line before the lock is taken again, so the release serves no purpose — nothing runs between the two acquisitions. What it does accomplish is splitting `CREATE OR REPLACE TABLE metadata_raw` + `CREATE OR REPLACE VIEW crate_dependencies` from the `count(*)` and `workspace_root` read of that same table into two critical sections. `record_count` and `workspace_root` are then written into the `data_sources` row (:62-71) alongside a checksum of the JSON file, so if anything replaces `metadata_raw` in the gap, the persisted provenance describes data that is no longer there.

Today the gap is mostly covered by accident: `provide_via_ingestor` (`extensions/duckdb/src/sql/ingest/orchestrator.rs:90-99`) holds a per-table ingest mutex across `collect` + `load`, so a second *metadata* ingest cannot interleave. That protection is external to this function, is keyed on the table name, and is not mentioned here — `load` is a public trait method on `DataIngestor` and nothing in its signature or docs says it may only be called under that mutex. The three extracted helpers (`build_views`, `query_record_count`, `extract_workspace_root`, added by FN-1 / TASK-1543) each take `&duckdb::Connection` rather than `&DuckDb` precisely so they can share one guard; the caller then declines to.

**Why it matters**: CONC-2 / atomicity. Low severity because the orchestrator's mutex makes it unreachable in the shipped call path, and because the fix is to delete two lines: hoist the single `let conn = db.lock()?;` above `build_views` and keep the existing `drop(conn)` at :59. The value is in removing a lock-release that reads as deliberate — a future reader has to go find the orchestrator to learn whether the gap matters.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 load() acquires the connection lock once and holds it across build_views, query_record_count and extract_workspace_root, dropping it before checksum_file/upsert_data_source as it does today
- [ ] #2 The dependency on the orchestrator's per-table ingest mutex is either removed by the single-guard change or documented on DataIngestor::load's contract
<!-- AC:END -->
