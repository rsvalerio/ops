---
id: TASK-1023
title: >-
  CONC-2: typed_manifest_cache eviction picks an arbitrary HashMap entry instead
  of LRU, can evict hot keys
status: Done
assignee: []
created_date: '2026-05-07 20:22'
updated_date: '2026-05-07 23:15'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:198-201`

**What**: When the bounded `typed_manifest_cache` reaches `MAX_TYPED_MANIFEST_CACHE_ENTRIES` (64) and a new key arrives, eviction is implemented as `if let Some(victim) = guard.keys().next().cloned() { guard.remove(&victim); }`. `HashMap::keys().next()` returns whichever bucket is first by hash order — there is no recency or frequency weighting. In a long-running daemon visiting many cwds (the very scenario the cap was added for, see TASK-0843 / CONC-2 comment), the steady-state hot key for the daemon's own workspace can be the one evicted while a one-shot transient cwd remains, defeating the cache's purpose.

**Why it matters**: TASK-0843's stated intent is "steady-state hits remain warm" — the current arbitrary eviction does not deliver that property. A daemon with 65+ cwds in rotation will thrash the cache instead of caching the hot subset.

**Suggested fix**: replace with an `IndexMap` and remove `[0]`, or add a per-entry `last_accessed` field and pick the oldest. The latter matches LRU semantics exactly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Eviction policy is documented (e.g. LRU or insertion-order) and the implementation matches
- [ ] #2 Unit test pins the eviction-victim selection so a future refactor cannot silently regress to arbitrary HashMap-order
<!-- AC:END -->
