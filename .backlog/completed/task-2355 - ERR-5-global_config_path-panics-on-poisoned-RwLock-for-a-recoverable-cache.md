---
id: TASK-2355
title: 'ERR-5: global_config_path panics on poisoned RwLock for a recoverable cache'
status: Done
assignee: []
created_date: '2026-10-04 14:09'
updated_date: '2026-10-04 15:51'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2423'
modified_files:
  - crates/core/src/config/loader/global.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-5:crates/core/src/config/loader/global.rs:global_config_path'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader/global.rs:73-95`

**What**: `global_config_path` (lines ~73-95) calls `.expect("GLOBAL_CONFIG_PATH lock poisoned")` on the read and write guard of `GLOBAL_CONFIG_PATH: RwLock<Option<Option<PathBuf>>>`. The protected state is a pure memoisation of `resolve_global_config_path()`; no invariant can be broken by a panicking holder (the value is either `None` -> recompute, or a fully-written `Some`). The sibling `crate::sync::lock_recover` documents exactly this case ("cache or dedup set with no broken invariant") and recovers instead of panicking, but there is no RwLock equivalent, so one panic in any thread that held the lock turns every later `load_config` call in a long-lived process into a panic. `reset_global_config_path_cache` (same file, ~line 146) has the same `expect`.

**Why it matters**: A production fn on the config-load path propagates an unrelated thread's panic; recovery via `PoisonError::into_inner` (and re-resolving) is cheap and removes the `#[allow(clippy::expect_used)]` justification entirely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 global_config_path no longer panics when GLOBAL_CONFIG_PATH is poisoned; it recovers the guard (into_inner) and re-resolves or reuses the cached value
- [x] #2 A test poisons the lock and asserts global_config_path still returns

<!-- AC:END -->
