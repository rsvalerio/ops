---
id: TASK-0780
title: >-
  ERR-5: ingest_mutex held across ingestor.collect+load with .unwrap()
  permanently bricks tables on panic
status: Triage
assignee: []
created_date: '2026-05-01 05:57'
labels:
  - code-review-rust
  - concurrency
  - errors
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:264`

**What**: `let _ingest_guard = ingest_mutex.lock().unwrap();` is held across user-supplied ingestor.collect(ctx, &data_dir)? and ingestor.load(...). A panic anywhere in collect/load poisons the per-table Arc<Mutex<()>>. Because the lock is only ever taken via .unwrap(), every subsequent provide_via_ingestor call for that same table panics for the lifetime of the process. The connection wrapper at connection.rs:91 deliberately maps poison to DbError::MutexPoisoned; this site diverges.

**Why it matters**: Turns a transient ingestor crash into a permanent denial of service for that data source. SEC-26 also calls out lock-related DoS risks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace unwrap() on both the outer map and per-table mutex locks with explicit recovery (e.g., into_inner() / clear poison or wrap as a typed error), or use parking_lot::Mutex which does not poison
- [ ] #2 Add a regression test where collect panics in one thread and verifies a subsequent caller still succeeds (or surfaces a typed error rather than a panic)
- [ ] #3 Cross-reference connection.rs MutexPoisoned policy in the call site comment
<!-- AC:END -->
