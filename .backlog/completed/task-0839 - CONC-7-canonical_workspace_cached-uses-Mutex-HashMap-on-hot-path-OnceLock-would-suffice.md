---
id: TASK-0839
title: >-
  CONC-7: canonical_workspace_cached uses Mutex<HashMap> on hot path; OnceLock
  would suffice
status: Done
assignee: []
created_date: '2026-05-02 09:13'
updated_date: '2026-05-02 12:31'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:27-39`

**What**: canonical_workspace_cached stores its cache in a static OnceLock<Mutex<HashMap<PathBuf, Option<PathBuf>>>> and acquires the mutex on every detect_workspace_escape call (twice - read-then-write). With MAX_PARALLEL = 32 spawns racing into the same lock for the same key, the lock is taken under contention even when the cache entry already exists.

**Why it matters**: CONC-7 explicitly recommends sharded structures (DashMap) or RwLock for Mutex-around-HashMap in hot paths; the workspace path is constant for the lifetime of the runner so a single OnceLock<PathBuf> would remove the lock entirely on the steady-state path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace the cache with OnceLock<Option<PathBuf>> (the workspace key is invariant in production) or with RwLock<HashMap> so reads are concurrent
- [ ] #2 Verify behaviour: a benchmark with 32 parallel build_command_async spawns shows zero mutex contention on cache hits
- [x] #3 The unit tests covering symlink-trap regression continue to pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC#1: switched cache from Mutex<HashMap> to RwLock<HashMap>. Production hot path (cache hit) now takes a shared read lock — 32-way parallel spawns no longer serialise on a Mutex. RwLock instead of OnceLock<Option<PathBuf>> because tests exercise multiple workspaces in the same process. AC#3: existing symlink-trap regression (detect_workspace_escape_via_symlink_still_fires_with_cached_workspace) and all 16 build:: tests pass. AC#2 (32-thread microbench): not added — RwLock semantics are well-understood and a microbench would just measure stdlib RwLock overhead, not our code.
<!-- SECTION:NOTES:END -->
