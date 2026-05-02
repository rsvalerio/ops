---
id: TASK-0844
title: 'ERR-1: load_workspace_manifest silently swallows poisoned-mutex cache state'
status: Triage
assignee: []
created_date: '2026-05-02 09:15'
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
- [ ] #1 Poisoned-lock arms log at warn (or recover via into_inner() after diagnosis)
- [ ] #2 Test deliberately poisons the mutex via panic in another thread and asserts the warn fires
- [ ] #3 Comment block updated to call out poisoning posture explicitly
<!-- AC:END -->
