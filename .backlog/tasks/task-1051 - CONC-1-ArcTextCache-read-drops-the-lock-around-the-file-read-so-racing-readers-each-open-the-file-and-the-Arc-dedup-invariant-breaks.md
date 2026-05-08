---
id: TASK-1051
title: >-
  CONC-1: ArcTextCache::read drops the lock around the file read, so racing
  readers each open the file and the Arc dedup invariant breaks
status: Done
assignee: []
created_date: '2026-05-07 21:02'
updated_date: '2026-05-07 21:10'
labels:
  - code-review
  - about
  - manifest_cache
  - CONC-1
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions/about/src/manifest_cache.rs:64-109 — ArcTextCache::read takes the mutex, does a get(), drops the guard, runs read_optional_text (file IO, potentially slow), then re-acquires the mutex to insert. Two threads racing for the same uncached path each observe get() == None, each call read_optional_text, each construct a distinct Arc<str>, and the second insert overwrites the first. The cache's documented contract (test second_call_returns_same_arc, line 124) — 'second call returns same Arc' — silently breaks under contention: callers that compared by Arc::ptr_eq (PERF-3 / TASK-0854 was the original motivation for handing Arcs out) regress to allocating duplicate buffers and losing pointer equality.

Suggested fix: use the entry API or get_or_insert_with under the lock, or accept that read_optional_text runs under the cache mutex (the cap is 1024 entries so the cache mutex is not held across more than one IO at a time per distinct path; a per-path OnceLock would also work). Either way the dedup contract should be a contention-safe invariant, not a sequential-only one.

Repro: spawn two threads that both call cache.read(same_root) simultaneously; assert Arc::ptr_eq(&a, &b). Today this fails intermittently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Concurrent reads of the same uncached path return Arc::ptr_eq() values
- [ ] #2 second_call_returns_same_arc behaviour is preserved for the sequential path
- [ ] #3 Regression test exercises the racing path
<!-- AC:END -->
