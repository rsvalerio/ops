---
id: TASK-2799
title: 'CONC-16: ArcTextCache recovers through poisoned mutexes guarding the LRU map and the filename registry'
status: Done
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 21:54'
labels:
  - code-review-rust
  - CONC
dependencies: []
parent_task_id: 'TASK-2839'
modified_files:
  - extensions/about/src/manifest_cache.rs
priority: medium
ordinal: 1000
dedup_key: 'CONC-16:extensions/about/src/manifest_cache.rs:ArcTextCache::read'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
<!-- scan confidence: policy-level finding; depends on the decision in TASK-2672 (ops_core::sync recovery policy) -->
**File**: `extensions/about/src/manifest_cache.rs:153-158` (`ArcTextCache::read`, `lock_recover_with`), `:212` (`ArcTextCache::invalidate`, `lock_recover_warn`), `:268` (`for_filename`, `lock_recover`)

**What**: All three lock sites use the `ops_core::sync::lock_recover*` helpers, which take `PoisonError::into_inner` and `clear_poison()`. The guarded `BoundedLruCache` is updated in several steps under the lock (`insert_filtered`: evict victim, `map.insert`, `push_stamp` with possible `retain_fresh` compaction), so a panic mid-update can leave the map and victim queue out of step — the case the docs' "the cache value is the raw file text, not authoritative state" justification does not cover. CONC-16 rejects that exemption explicitly.

**Why it matters**: Recovering hands later readers possibly half-updated cache state and erases the evidence of the panic. These are caller sites of the core policy tracked in TASK-2672; resolve them together so ops-about does not keep recovering after the core policy changes (or record the exemption here too).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Per the TASK-2672 decision, manifest_cache.rs lock sites either propagate poison (.lock().expect(...)) or cite the recorded exemption
- [x] #2 Poison-recovery tests in manifest_cache.rs are updated to the chosen policy

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).

Following the TASK-2672 decision (propagate), read, invalidate and for_filename use ops_core::sync::lock, with the panic naming the filename. poison_recovery_keeps_cache_usable is replaced by poisoned_cache_lock_is_propagated, which asserts that read and invalidate both panic naming manifest.txt and that the poison is not cleared.

<!-- SECTION:NOTES:END -->
