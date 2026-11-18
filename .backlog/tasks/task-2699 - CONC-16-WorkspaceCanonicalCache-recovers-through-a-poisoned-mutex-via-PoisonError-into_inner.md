---
id: TASK-2699
title: 'CONC-16: WorkspaceCanonicalCache recovers through a poisoned mutex via PoisonError::into_inner'
status: Done
assignee: []
created_date: '2026-11-18 18:30'
updated_date: '2026-11-18 21:54'
labels:
  - code-review-rust
  - CONC
dependencies: []
parent_task_id: 'TASK-2839'
modified_files:
  - crates/runner/src/command/build.rs
priority: medium
ordinal: 1000
dedup_key: 'CONC-16:crates/runner/src/command/build.rs:recover_workspace_cache'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:191` (`recover_workspace_cache`), called from `WorkspaceCanonicalCache::invalidate` (`build.rs:90`), `WorkspaceCanonicalCache::clear` (`build.rs:99`) and `WorkspaceCanonicalCache::get_or_compute` (`build.rs:138`)

**What**: Every lock of the runner's workspace canonicalize cache is `self.inner.lock().unwrap_or_else(|e| recover_workspace_cache(e))`, and `recover_workspace_cache` logs a warning and returns `err.into_inner()`. A panic while the guard is held (for example inside the caller-supplied `canonicalize` closure that `get_or_compute` runs under the lock, after it has already evicted the LRU victim) is therefore silently continued past. This is the runner-crate twin of the ops-core poison-recovery policy filed as TASK-2672.

**Why it matters**: The cache decides cwd-escape outcomes for every spawn (`detect_workspace_escape` under `WarnAndAllow`). CONC-16 forbids reaching inside a poisoned lock: "this lock guards no multi-step invariant" is not an exemption, because `get_or_compute` already does evict-then-compute-then-insert under one guard, and nothing re-checks the claim when that section grows. Propagating (`.lock().expect("workspace canonicalize cache poisoned")`) surfaces the failure through the existing `spawn_blocking` JoinError path in `build_command_async`, which already converts a build panic into a step failure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No production lock of WorkspaceCanonicalCache uses PoisonError::into_inner/get_ref/get_mut or clear_poison; poisoning propagates (e.g. .lock().expect(...)), or a recorded decision (shared with TASK-2672) documents the exemption
- [x] #2 A test poisons the cache mutex and asserts the next spawn surfaces a step failure rather than silently continuing

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.05 (code-review-rust run, 2026-11-18).

The WorkspaceCanonicalCache locks now go through ops_core::sync::lock, and recover_workspace_cache is deleted. Test build::tests::poisoned_workspace_cache_surfaces_as_a_step_failure poisons the cache, runs exec_command with a spec cwd, and asserts a failed step carrying the BuildTaskFailed message. It also asserts the poison stays set.

<!-- SECTION:NOTES:END -->
