---
id: TASK-1073
title: >-
  CONC-2: duckdb provide_via_ingestor releases ingest mutex before query_fn,
  racing concurrent --refresh DROP
status: Done
assignee: []
created_date: '2026-05-07 21:19'
updated_date: '2026-05-08 06:37'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:367-380`

**What**: The per-table ingest mutex spans collect+load (correct for dedup), but `query_fn(db)` at line 381 runs lock-free. A second thread entering with `ctx.refresh = true` between line 380 and line 381 calls `drop_table_if_exists` while the first thread is mid-query, producing an opaque DuckDB "table not found" error rather than the documented happy path.

**Why it matters**: Concurrent `ops about --refresh` (or two providers ingesting the same table in one invocation) can crash one query with a misleading error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Hold the ingest mutex across query_fn (preferred), OR document the race as a known limitation in rustdoc with a regression test exhibiting the failure mode
- [x] #2 Verify the connection-level lock inside query_rows_to_json cannot interleave a DROP between prepare and query_map
<!-- AC:END -->
