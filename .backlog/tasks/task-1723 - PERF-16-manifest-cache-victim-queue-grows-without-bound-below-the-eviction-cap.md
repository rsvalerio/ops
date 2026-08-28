---
id: TASK-1723
title: >-
  PERF-16: manifest cache victim queue grows without bound below the eviction
  cap
status: To Do
assignee:
  - TASK-2003
created_date: '2026-08-27 11:11'
updated_date: '2026-08-28 14:15'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - extensions/about/src/manifest_cache.rs
  - extensions/about/src/lru.rs
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/manifest_cache.rs:200` and `extensions/about/src/manifest_cache.rs:237` (push sites), `extensions/about/src/manifest_cache.rs:102` (`evict_lru`, the only drain site), `extensions/about/src/lru.rs:66` (`LruVictimQueue::push`)

**What**: `ArcTextCache::read` pushes a fresh `(tick, path)` pair onto `CacheMap::victim_queue` on **every** call — both on the cache-hit path (line 197-200) and on insert (line 229-237). The only code that ever pops from that heap is `LruVictimQueue::pop_lru`, called exclusively from `CacheMap::evict_lru`, which `read` invokes only inside `if guard.len() >= CACHE_MAX_ENTRIES` (line 218).

Consequence: as long as the map holds fewer than `CACHE_MAX_ENTRIES` (1024) distinct paths — the overwhelmingly common case — the heap is **never** drained, and it accumulates one `Reverse((u64, PathBuf))` per read forever. A process that reads three manifests in a loop keeps a `map` of size 3 and a `victim_queue` that grows linearly with the number of reads. The lazy-invalidation design assumes the eviction sweep runs often enough to discard stale heads; below the cap it never runs at all.

The module docs name the affected consumers explicitly: "long-running embedders (LSP-style hosts, watchers)" re-entering paths "at a steady rate". Those are exactly the processes where this grows unbounded. A short-lived `ops` CLI invocation is unaffected, which is why the existing tests (`cap_eviction_drops_lru_not_whole_map`, `concurrent_distinct_path_reads_do_not_block_each_other`) do not see it.

Secondary gap in the same cache policy: the cache has a stated key, value and maximum size, but **no TTL and no invalidation path at all**. Once `<root>/<filename>` is read, the `Arc<str>` is pinned for the process lifetime with no mtime check and no explicit invalidate API, so a watcher host serving an edited `package.json` keeps returning the pre-edit text indefinitely.

Cross-crate note: the sibling `typed_manifest_cache` in `extensions-rust/about/src/query.rs` shares `crate::lru` and has the same push-on-hit shape, so the fix belongs in `ops-about` (either in `LruVictimQueue` itself or in the shared push discipline) rather than being patched per consumer.

**Why it matters**: unbounded memory growth in the exact deployment shape the module was written for. It is invisible in CLI use and only surfaces as a slow leak in a daemon, which is the hardest place to diagnose it. The missing invalidation compounds it: the daemon both leaks and serves stale data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 victim_queue length is bounded by a function of map size (e.g. drain stale heads opportunistically on hit, or cap the heap and rebuild, or replace push-on-hit with an in-place tick update)
- [ ] #2 A regression test warms a small number of distinct roots, performs many repeated reads well below CACHE_MAX_ENTRIES, and asserts the victim queue does not grow linearly with read count
- [ ] #3 The cache's TTL / invalidation expectation is either implemented (mtime or explicit invalidate) or documented as 'none by design' with the stale-read consequence stated for the long-running-embedder consumers named in the module docs
<!-- AC:END -->
