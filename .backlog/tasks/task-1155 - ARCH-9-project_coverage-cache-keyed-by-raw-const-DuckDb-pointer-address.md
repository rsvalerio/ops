---
id: TASK-1155
title: 'ARCH-9: project_coverage cache keyed by raw *const DuckDb pointer address'
status: To Do
assignee:
  - TASK-1261
created_date: '2026-05-08 07:43'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:43`

**What**: `cached_query_project_coverage` keys a process-global Mutex<HashMap<usize, …>> by `std::ptr::from_ref(db) as usize`. Comment claims \"naturally process-scoped and never has to invalidate\" but the cache is `&'static`; nothing prevents the address from being reused if a DuckDb Arc is dropped and a new one allocated at the same address (e.g. test harness opening multiple in-memory DBs in sequence).

**Why it matters**: ABA on raw pointers as cache keys is a classic correctness hazard. Today it works only because `ops about` runs once per process; any future change reusing a Context across DB instances (CLI daemon, integration fixtures) silently returns stale results from a previous DB. Tests already need an explicit clear hook.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace pointer-as-key with an explicit identity attached to DuckDb (Arc::as_ptr of held connection, or a cheap DuckDbId(u64) minted on open)
- [ ] #2 Or scope the cache to Context (one Mutex<Option<…>> per request) so the lifetime is the request not the process
- [ ] #3 Drop clear_project_coverage_cache_for_test once the cache key is no longer ambient
<!-- AC:END -->
