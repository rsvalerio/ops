---
id: TASK-0843
title: >-
  CONC-2: typed_manifest_cache in extensions-rust is unbounded and has no mtime
  invalidation
status: Triage
assignee: []
created_date: '2026-05-02 09:14'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:55-99`

**What**: typed_manifest_cache() returns a Mutex<HashMap<PathBuf, Arc<CargoToml>>>. The cache has no eviction policy and the entry is never refreshed unless ctx.refresh = true. A long-running daemon process accumulates entries keyed by every working directory ever visited, and a stale Cargo.toml mtime is never noticed.

**Why it matters**: Memory leak in long-lived processes plus stale-data hazard. The CACHE comment claims contention is bounded but does not mention growth or staleness.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cache entries are bounded (LRU with cap, or evicted on file mtime change)
- [ ] #2 Or the cache is documented as request-scoped only, and is cleared at process boundary or after N inserts
- [ ] #3 Test asserts the chosen invariant (size-cap or mtime invalidation)
<!-- AC:END -->
