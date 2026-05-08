---
id: TASK-1240
title: >-
  PERF-1: ArcTextCache LRU victim selection scans the entire HashMap on every
  cap-bound miss
status: To Do
assignee:
  - TASK-1263
created_date: '2026-05-08 12:59'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/manifest_cache.rs:126-138`

**What**: When `guard.len() >= CACHE_MAX_ENTRIES` (1024) the cache picks the victim via `iter().min_by_key(...)`, paying O(n) on top of the file read for every cap-bound miss. The sibling `typed_manifest_cache` (kept in lockstep per the module-level docs) inherits the same shape.

**Why it matters**: Long-running embedders that thrash distinct roots above the cap pay ~1024 HashMap probes per evicting read on top of IO; an `lru`/`linked-hash-map` or BinaryHeap victim queue brings this to O(log n) or O(1).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the linear min_by_key scan with an O(log n) / O(1) eviction queue
- [ ] #2 Apply the same change to typed_manifest_cache per the existing lockstep contract
- [ ] #3 Microbench / alloc-counter test pinning sub-linear eviction cost
<!-- AC:END -->
