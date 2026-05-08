---
id: TASK-1106
title: >-
  ARCH-1: ArcTextCache cap eviction clears the entire map (full-flush, not LRU);
  sibling drift hazard with TASK-1023
status: Done
assignee: []
created_date: '2026-05-07 21:34'
updated_date: '2026-05-08 06:51'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/manifest_cache.rs:90-97`

**What**: When the cache reaches `CACHE_MAX_ENTRIES` (1024) it calls `guard.clear()` — every cached `Arc<str>` is dropped. Long-running embedders (LSP-style hosts, watchers) that re-enter `ops_about` paths at a steady rate pay the full re-read cost in unison after each eviction storm. TASK-1023 covers a different cache (`typed_manifest_cache`) with the same anti-pattern; this is a sibling site that must move in lockstep.

**Why it matters**: Two caches with divergent eviction policies will silently drift; need a shared decision so behaviour is uniform.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace full-clear with at least a 'drop oldest N' or LRU policy shared with the fix for TASK-1023
- [x] #2 Test: warm 1024 distinct paths, then read a 1025th — at least one previously-cached path should still return its Arc<str> if LRU, or all should miss if random — pin whichever choice is made
- [x] #3 Add a doc note cross-referencing TASK-1023 so the two caches do not drift again
<!-- AC:END -->
