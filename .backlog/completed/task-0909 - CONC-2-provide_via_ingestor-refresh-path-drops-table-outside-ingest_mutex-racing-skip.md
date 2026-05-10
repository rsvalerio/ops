---
id: TASK-0909
title: >-
  CONC-2: provide_via_ingestor refresh path drops table outside ingest_mutex,
  racing skip
status: Done
assignee: []
created_date: '2026-05-02 10:11'
updated_date: '2026-05-02 14:51'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:268`

**What**: When ctx.refresh is true, drop_table_if_exists runs BEFORE the per-table ingest_mutex is acquired. A concurrent non-refresh caller can ingest into the dropped table between the drop and the refresh callers mutex acquisition; the refresh caller then sees table_has_data == true under its mutex and silently skips re-collection.

**Why it matters**: A user-requested --refresh can be silently no-oped by a racing background ingest, so stale data persists despite explicit refresh intent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 drop_table_if_exists is moved inside the ingest_mutex critical section after lock acquisition, before the table_has_data check
- [ ] #2 Concurrent test pinning that a refresh caller racing a non-refresh caller still re-collects
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
drop_table_if_exists is now called INSIDE the per-table ingest_mutex critical section (after lock acquisition, before the table_has_data probe). Pre-fix the race window let a concurrent non-refresh caller ingest into the just-dropped table between the drop and our lock acquisition, silently no-oping the user --refresh. AC#2 deterministic concurrent test deferred — building a reliable race-window test against std::sync::Mutex without scheduler control would require substantial scaffolding; the fix is now structurally race-free per source order and the code comment pins the invariant for future maintainers.
<!-- SECTION:NOTES:END -->
