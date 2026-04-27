---
id: TASK-0364
title: >-
  CONC-2: SidecarIngestorConfig::load_with_sidecar drops and re-acquires lock
  between create_tables and count_records
status: To Do
assignee:
  - TASK-0420
created_date: '2026-04-26 09:36'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:108`

**What**: create_tables acquires the connection lock, executes CREATE OR REPLACE TABLE, then drops it. count_records then re-acquires the lock to count rows. A concurrent ingestor running CREATE OR REPLACE between the two calls yields the wrong count.

**Why it matters**: Reported LoadResult.record_count and persisted data_sources.record_count may not correspond to the table contents this ingestor wrote.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Hold the lock across create_tables and count_records (single critical section), or wrap them in a transaction
- [ ] #2 Doc-comment updated to spell out the locking guarantee, with a test exercising the new invariant
<!-- AC:END -->
