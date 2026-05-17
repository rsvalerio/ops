---
id: TASK-1470
title: >-
  ERR-5: stack/detect.rs uses .expect on Mutex lock instead of poison-recover
  policy
status: Done
assignee:
  - TASK-1480
created_date: '2026-05-16 10:06'
updated_date: '2026-05-17 07:56'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack/detect.rs:28,46`

**What**: Two production-path `cache.lock().expect("canonicalize cache poisoned")` calls will panic if any prior caller panicked while holding the lock, even though the cached state (HashMap<PathBuf, PathBuf>) has no invariant the panic could have broken.

**Why it matters**: Every other Mutex in this crate (expand.rs:160, expand.rs:228, expand.rs:248) uses the `unwrap_or_else(|e| { cache.clear_poison(); e.into_inner() })` recovery pattern documented as CONC-9. The detect path is the odd one out: a single poisoned lock here turns every subsequent Stack::detect into a hard panic for the rest of the process, which is reachable from production CLI dispatch.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace both expect calls (and the #[cfg(test)] reader at line 62) with the same clear_poison + into_inner pattern used in expand.rs
- [ ] #2 Document the consistent policy in module-level comments or factor a lock_recover helper shared between stack/detect.rs and expand.rs
- [ ] #3 Add a regression test that pollutes the cache via a panicking thread, then asserts the next detect call still resolves rather than panicking
<!-- AC:END -->
