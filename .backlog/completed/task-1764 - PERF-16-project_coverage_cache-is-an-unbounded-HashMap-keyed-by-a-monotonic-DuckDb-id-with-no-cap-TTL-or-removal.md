---
id: TASK-1764
title: >-
  PERF-16: project_coverage_cache is an unbounded HashMap keyed by a monotonic
  DuckDb id with no cap, TTL, or removal
status: Done
assignee:
  - TASK-1993
created_date: '2026-08-27 11:20'
updated_date: '2026-08-28 20:09'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - extensions-rust/about/src/coverage_provider.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:37` (`project_coverage_cache`), populated at `:57-69` (`cached_query_project_coverage`)

**What**: The memoization introduced by DUP-1 / TASK-1079 is a bare `static Mutex<HashMap<u64, Arc<OnceLock<Option<CrateCoverage>>>>>`. Entries are only ever inserted (`guard.entry(key).or_insert_with(...)` at `:65-68`). There is no size cap, no TTL, no eviction, and no removal when the `DuckDb` that minted the key is dropped — the cache's own test acknowledges this at `:383-385`: *"The slot under `a_id` may still exist (the cache is keyed by id, not by liveness)"*.

Because `DuckDb::id()` is a monotonic counter minted per instance (the ABA fix from ARCH-9 / TASK-1155), every `DuckDb` a process ever opens leaves a permanent entry behind. A key never repeats, so the map is strictly monotonic in the number of `DuckDb` instances opened for the lifetime of the process.

Contrast with the sibling cache in the same crate: `query.rs:30` gives `typed_manifest_cache` a `MAX_TYPED_MANIFEST_CACHE_ENTRIES = 64` cap with LRU eviction (`TypedManifestCache::evict_lru`), for exactly the daemon/CI-worker deployment shape `query.rs:248-272` documents. `project_coverage_cache` guards the same process against the same hosts with none of that.

Per PERF-16, a cache needs a stated key, value, maximum size, TTL, and invalidation expectation. This one has a key and a value; the other three are absent, and the leak is unbounded rather than capped.

**Why it matters**: In the single-shot `ops about` CLI the map holds one entry and nothing is visible. In a long-running host that opens a `DuckDb` per project or per refresh — precisely the daemon shape `query.rs` was hardened for — the map grows one `CrateCoverage`-sized slot per instance forever, and the entries pin nothing useful because the `DuckDb` they describe is already gone. It is also a correctness smell: the memoized value can never be invalidated, so a re-ingested coverage table behind the *same* `DuckDb` handle keeps serving the pre-ingest number.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 project_coverage_cache has a stated maximum size and an eviction or removal policy (entry removed when its DuckDb drops, or a bounded LRU matching the typed_manifest_cache pattern in query.rs)
- [x] #2 The cache's invalidation expectation is either implemented or documented in the module docs, stating what happens when coverage data is re-ingested behind a live DuckDb handle
- [x] #3 A regression test opens many DuckDb instances, calls cached_query_project_coverage for each, and asserts the cache map length stays bounded
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
project_coverage_cache is now a ProjectCoverageCache (HashMap + LruVictimQueue) capped at MAX_COVERAGE_CACHE_ENTRIES = 16 with LRU eviction on insert, mirroring the manifest_cache policy. The type doc states key, value, maximum size and the invalidation expectation explicitly - including that data re-ingested behind a live DuckDb handle keeps serving the pre-ingest number and that a fresh handle mints a fresh key. Regression test: project_coverage_cache_stays_bounded_across_many_db_instances opens 3x the cap and asserts the map length stays within it.
<!-- SECTION:NOTES:END -->
