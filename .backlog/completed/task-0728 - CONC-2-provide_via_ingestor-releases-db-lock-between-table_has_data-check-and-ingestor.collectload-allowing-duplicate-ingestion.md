---
id: TASK-0728
title: >-
  CONC-2: provide_via_ingestor releases db lock between table_has_data check and
  ingestor.collect+load, allowing duplicate ingestion
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 05:48'
updated_date: '2026-04-30 18:42'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:215-240`

**What**: `provide_via_ingestor` calls `table_has_data(db, table_name)?` (which acquires-and-releases the lock), then drops the lock, then runs `ingestor.collect(ctx, &data_dir)?`, `init_schema(db)?`, and `ingestor.load(...)?`. Two concurrent invocations against the same `(db, table_name)` can both observe `table_has_data == false` and both proceed to run an external collect (e.g. `cargo metadata`, full `tokei` filesystem scan) and then race a `CREATE OR REPLACE TABLE` inside `load`. The `load_with_sidecar` lock-holding fix from TASK-0364 protects the create→count sub-section but not the outer "should I collect at all" decision.

**Why it matters**: collect can be expensive (multi-second tokei scan, cargo subprocess) and side-effecting (writes JSON sidecars). Doing it twice wastes CPU and wall-clock, and the second writer can race the first on `data_dir/<name>.json`. The previous `LoadResult::record_count` is also discarded so the duplicate is invisible in logs. Mitigation: hold the connection lock across the existence check and ingest, or use a per-table `Once`/named mutex keyed by `(db_path, table_name)` so only one ingest can be in flight.

**Why this differs from TASK-0364**: that fix scoped the lock around create→count *inside* `load_with_sidecar`. The race here is one level up — between the `table_has_data` decision and entering `load_with_sidecar` at all.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Concurrent provide_via_ingestor calls for the same table run collect at most once, or the redundant collect is shown to be harmless and the contract is documented
- [ ] #2 Regression test exercises two threads invoking provide_via_ingestor against an empty table and asserts collect runs once
<!-- AC:END -->
