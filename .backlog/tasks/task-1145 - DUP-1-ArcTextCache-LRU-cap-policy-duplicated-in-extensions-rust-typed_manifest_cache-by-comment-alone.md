---
id: TASK-1145
title: >-
  DUP-1: ArcTextCache LRU/cap policy duplicated in extensions-rust
  typed_manifest_cache by comment alone
status: To Do
assignee:
  - TASK-1265
created_date: '2026-05-08 07:42'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/manifest_cache.rs:27-30`

**What**: Module doc states the LRU/cap policy \"MUST be kept in lockstep with the sibling typed_manifest_cache in extensions-rust/about/src/query.rs (TASK-1023)\" or \"the two caches will silently drift\". The contract has no compile-time or test-level enforcement — CACHE_MAX_ENTRIES, the next_lru_tick monotonic stamp, the poison-recovery shape, and the eviction policy all exist twice with synchronisation by code review.

**Why it matters**: Exactly the regression shape DUP-1 names: the next eviction-policy fix lands in one copy and not the other; no signal to reviewers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a generic ArcTextCache<T> into ops_about (or shared crate) parameterised by deserialiser; both crates depend on it
- [ ] #2 Or pin CACHE_MAX_ENTRIES in both modules to a single shared const and add an integration test asserting identical eviction
<!-- AC:END -->
