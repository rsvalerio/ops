---
id: TASK-1143
title: >-
  CONC-2: provide_via_ingestor non-reentrant ingest guard spans query_fn —
  same-table reentry deadlocks
status: To Do
assignee:
  - TASK-1261
created_date: '2026-05-08 07:41'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:357-409`

**What**: `_ingest_guard` (a std::sync::Mutex<()> guard from db.ingest_mutex_for(table_name)) is held across `query_fn(db)` by design (CONC-2 / TASK-1073). std::sync::Mutex is non-reentrant. A query_fn that recursively calls provide_via_ingestor for the same table_name (e.g. a future provider fanning out, or a reentrant degraded-fallback) deadlocks the thread silently. Only defense today is a code comment.

**Why it matters**: Latent self-deadlock under a plausible refactor; no compile-time or runtime signal would catch the re-entry. Contract lives in a comment, not the type system.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace bare lock() with try_lock() returning a typed IngestInProgress error so reentrance surfaces as an error
- [ ] #2 Or document the non-reentrancy contract on the function's public rustdoc and add a debug_assert tracking the holding ThreadId
<!-- AC:END -->
