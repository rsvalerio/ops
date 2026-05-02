---
id: TASK-0844
title: 'ERR-1: load_workspace_manifest silently swallows poisoned-mutex cache state'
status: Done
assignee: []
created_date: '2026-05-02 09:15'
updated_date: '2026-05-02 14:08'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:71-97`

**What**: Three `if let Ok(... guard) = cache.lock()` arms swallow PoisonError. If a panic in another provider poisons the mutex, the cache silently degrades to "always-miss" without any signal - exactly the failure mode the file preceding comment (the thread_local! regression) warns against.

**Why it matters**: A poisoned cache is invisible. Operators see degraded performance with no log line; the regression that motivated the rewrite (TASK-0795) is reproduced through a different mechanism.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Poisoned-lock arms log at warn (or recover via into_inner() after diagnosis)
- [x] #2 Test deliberately poisons the mutex via panic in another thread and asserts the warn fires
- [x] #3 Comment block updated to call out poisoning posture explicitly
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Cache lock now goes through lock_typed_manifest_cache: PoisonError → warn (one-shot via OnceLock) → into_inner() → clear_poison(). Cache values are plain data so into_inner is safe. Module comment updated to call out the poisoning posture explicitly. Regression test typed_manifest_cache_recovers_from_poison_with_warn deliberately poisons the mutex via a thread that panics inside the held lock and asserts recovery; serial_test::serial(typed_manifest_cache) added to the cache-touching tests so they don't race the poison test.
<!-- SECTION:NOTES:END -->
