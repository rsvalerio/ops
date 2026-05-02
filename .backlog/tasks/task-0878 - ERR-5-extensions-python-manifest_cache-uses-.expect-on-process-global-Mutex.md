---
id: TASK-0878
title: 'ERR-5: extensions-python manifest_cache uses .expect on process-global Mutex'
status: Triage
assignee: []
created_date: '2026-05-02 09:24'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/manifest_cache.rs:31,50`

**What**: Two cache.lock().expect("pyproject cache poisoned") calls in production code. Mutex poisoning means a previous holder panicked; in this read-mostly cache the data is still consistent.

**Why it matters**: A panic in one provider can permanently disable the cache process-wide via a re-panic, even though the cache state is recoverable. The other panic-on-poison sites in this workspace are guarded by unwrap_or_else(|e| e.into_inner()) (see existing tasks 0750, etc.).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace both .expect(...) calls with .unwrap_or_else(|e| e.into_inner()), or migrate to parking_lot::Mutex (no poisoning)
- [ ] #2 Add a test that panics inside a closure holding the lock and verifies subsequent calls still succeed
<!-- AC:END -->
