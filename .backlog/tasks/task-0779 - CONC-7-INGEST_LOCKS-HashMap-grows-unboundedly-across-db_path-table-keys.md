---
id: TASK-0779
title: 'CONC-7: INGEST_LOCKS HashMap grows unboundedly across (db_path, table) keys'
status: Triage
assignee: []
created_date: '2026-05-01 05:57'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:259`

**What**: Static OnceLock<Mutex<HashMap<String, Arc<Mutex<()>>>>> in provide_via_ingestor keeps an Arc<Mutex> entry per "{db_path}:{table_name}" forever. Entries are never removed, even when DBs close.

**Why it matters**: Long-running processes (daemons, repeated workspace switches) accumulate entries indefinitely. CONC-7/PERF guidance discourages global Mutex<HashMap> hot-path locks; here it's also a slow leak. Combined with unwrap-on-poison, the leak is a permanent failure mode.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document or bound the cache (size cap, LRU, or per-DuckDb instance map) so it does not grow with workspace switches
- [ ] #2 Add a regression test exercising N distinct (db_path, table) keys and verifying memory or count stays bounded
<!-- AC:END -->
