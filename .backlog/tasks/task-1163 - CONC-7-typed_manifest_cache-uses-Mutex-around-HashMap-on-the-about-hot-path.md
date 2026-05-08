---
id: TASK-1163
title: 'CONC-7: typed_manifest_cache uses Mutex around HashMap on the about hot path'
status: Done
assignee:
  - TASK-1261
created_date: '2026-05-08 07:45'
updated_date: '2026-05-08 14:46'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:146`

**What**: Process-global `Mutex<HashMap<PathBuf, TypedManifestEntry>>` covers the manifest cache. `load_workspace_manifest` is called by every about provider per `ops about` invocation, each holding the lock during HashMap mutation + LRU scan. Eviction is O(N) under the held lock (lines 264-271).

**Why it matters**: CONC-7 forbids Mutex around collections in hot paths. For single-shot CLI this is fine (low contention). The cache is documented as supporting daemon/language-server hosts; under that intended workload, multiple worker threads contend on this single mutex.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document the concurrency contract explicitly: Mutex is OK for single-shot CLI; switch to DashMap when daemon mode lands
- [ ] #2 Or pre-emptively adopt DashMap<PathBuf, TypedManifestEntry> and a separate parking_lot::Mutex<()> only for LRU eviction scan
<!-- AC:END -->
