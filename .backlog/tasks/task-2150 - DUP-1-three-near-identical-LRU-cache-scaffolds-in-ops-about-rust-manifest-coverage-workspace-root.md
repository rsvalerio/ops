---
id: TASK-2150
title: 'DUP-1: three near-identical LRU cache scaffolds in ops-about-rust (manifest, coverage, workspace-root)'
status: Done
assignee: []
created_date: '2026-09-08 07:03'
updated_date: '2026-09-10 16:01'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2242'
modified_files:
  - extensions-rust/about/src/manifest_cache.rs
  - extensions-rust/about/src/coverage_provider.rs
  - extensions-rust/about/src/workspace_root_cache.rs
priority: medium
ordinal: 63000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/manifest_cache.rs:100-297`, `extensions-rust/about/src/coverage_provider.rs:57-150`, `extensions-rust/about/src/workspace_root_cache.rs:54-158`

**What**: The crate carries three hand-written LRU caches that are the same construct with different key/value types. Each one repeats, near-verbatim:

- a `struct { map: HashMap<K, Entry>, victim_queue: LruVictimQueue<K> }` pair;
- a private `const *_VICTIM_QUEUE_SLACK: usize = 16` (`manifest_cache::VICTIM_QUEUE_SLACK`, `coverage_provider::COVERAGE_VICTIM_QUEUE_SLACK`, `workspace_root_cache::VICTIM_QUEUE_SLACK`);
- an identical `fn record_access(&mut self, key, tick)` — `victim_queue.push(tick, key)`, then `map.len().saturating_mul(2).saturating_add(SLACK)`, then `retain_fresh(|k, t| map.get(..).is_some_and(|e| e.last_accessed == t))`. The three bodies differ only in `map.get(key)` vs `map.get(key.as_ref())`;
- an identical `fn evict_lru(&mut self)` — `victim_queue.pop_lru(<same freshness closure>)` then `map.remove(..)`;
- the same cap-check-then-evict preamble on the insert path (`if !map.contains_key(k) && map.len() >= MAX_* { evict_lru() }`);
- the same "bump the tick, clone the Arc, `record_access` after the map update" hit path.

On top of that, `manifest_cache::lock_typed_manifest_cache` and `workspace_root_cache::lock` are two copies of the same poison-recovering `lock()` helper (`PoisonError::into_inner` + `clear_poison`), differing only in whether they warn; `coverage_provider` inlines a third, terser copy.

`ops_about::lru` already owns `LruVictimQueue` / `next_lru_tick`, so the shared *primitives* exist — what is duplicated is the cache scaffolding built on top of them. The sibling `extensions/about/src/manifest_cache.rs` is a fourth instance, and the module docs already ask reviewers to keep the copies "in lockstep" by hand (`manifest_cache.rs:104-107`, `:133-135`), which is the maintenance cost this finding is about.

**Why it matters**: Every past fix to this construct had to be applied three-to-four times: TASK-1723 (victim-queue compaction), TASK-1572 (`Arc` keys), TASK-1240 (heap eviction), TASK-1023 (tick-on-hit). A future fix that lands in two of the three is a silent per-cache behaviour divergence — an unbounded victim queue or a wrong eviction victim in whichever copy was missed — with no compile error and no test to catch it, because each copy is tested separately against its own bound. The "keep in lockstep" comment is a documented manual invariant standing in for a type.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A single generic bounded-LRU cache type (e.g. `ops_about::lru::BoundedLruCache<K, V>`) owns the map + victim queue, the compaction threshold and slack, `record_access`, `evict_lru`, and the cap-check-then-evict insert preamble
- [x] #2 `manifest_cache::TypedManifestCache`, `coverage_provider::ProjectCoverageCache` and `workspace_root_cache::WorkspaceRootCache` are expressed in terms of that type and no longer define their own `record_access` / `evict_lru` / slack constant
- [x] #3 The poison-recovering lock helper exists once (with the warn behaviour parameterised) rather than once per cache module
- [x] #4 The existing per-cache tests (cap eviction, LRU victim choice, victim-queue boundedness below the cap) still pass unchanged, and the shared type carries one set of those tests directly
- [x] #5 The "keep in lockstep with the sibling cache" review comments in `manifest_cache.rs` are removed or reduced to a pointer at the shared type

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Finished by resumed runner after API-limit kill: shared BoundedLruCache<K, V, Q> + lock_recovering live in ops_about::lru (extensions/about/src/lru.rs) with the per-cache LRU tests carried against the shared type; all three extensions-rust/about caches migrated, lockstep comments there removed. The fourth instance (extensions/about ArcTextCache) was deliberately left on LruVictimQueue primitives: its eviction pins in-flight OnceLock entries (CONC-1 / TASK-1144), a policy the shared type does not model — Triage task filed rather than forcing an invasive extension mid-wave. ops-about + ops-about-rust: 106 tests green.
<!-- SECTION:NOTES:END -->
