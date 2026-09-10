---
id: TASK-2257
title: 'DUP-1: ArcTextCache (extensions/about) still hand-rolls the LRU scaffold BoundedLruCache now owns'
status: Triage
assignee: []
created_date: '2026-09-10 16:27'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions/about/src/manifest_cache.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/manifest_cache.rs:79` (`CacheMap`)

**What**: TASK-2150 consolidated the LRU cache scaffold (map + victim queue, compaction slack, record/evict loop, cap-check-then-evict insert preamble) into `ops_about::lru::BoundedLruCache` and migrated the three `extensions-rust/about` caches. The fourth instance named by TASK-2150 — `ArcTextCache::CacheMap` in `extensions/about/src/manifest_cache.rs` — still carries its own `record_access` / `evict_lru` / `VICTIM_QUEUE_SLACK` on the raw `LruVictimQueue` primitives.

**Why it matters**: The scaffold now exists twice again, and the copies are in the same crate as the shared type. The blocker is real but bounded: `CacheMap::evict_lru` pins in-flight entries (an entry whose per-key `OnceLock` has not been initialised yet is skipped as a victim and pushed back with its original tick, CONC-1 / TASK-1144) — a policy `BoundedLruCache` does not model. Migrating requires extending the shared type with an eviction-candidate filter (e.g. a `can_evict: impl Fn(&V) -> bool` hook or a `push_back` path) and threading `ArcTextCache`'s get-or-insert flow through it, plus moving its tests onto the shared type.

**Origin**: discovered during TASK-2242 (wave 8) while fixing TASK-2150; left out of the wave because the pinning-hook extension is a design change to a freshly landed shared type, not a mechanical migration.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CacheMap is expressed in terms of ops_about::lru::BoundedLruCache (or a successor API in ops_about::lru) and no longer defines its own record_access / evict_lru / VICTIM_QUEUE_SLACK
- [ ] #2 The in-flight-entry pinning policy (CONC-1 / TASK-1144) is preserved: an entry whose OnceLock is uninitialised is never an eviction victim
- [ ] #3 The existing ArcTextCache tests (cap eviction, LRU victim choice, victim-queue boundedness, ptr_eq dedup) still pass
<!-- AC:END -->
