---
id: TASK-1477
title: >-
  DUP-3: Mutex poison-recover pattern is open-coded at 4+ sites; should factor
  to shared helper
status: Done
assignee:
  - TASK-1480
created_date: '2026-05-16 10:06'
updated_date: '2026-05-17 07:56'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:148-154,228-234,248-254` and `crates/core/src/stack/detect.rs:25-33,42-47`

**What**: The `let mut guard = cache.lock().unwrap_or_else(|e| { cache.clear_poison(); e.into_inner() })` block (including the CONC-9 comment) is open-coded at four sites; combined with the sibling ERR-5 finding this is also why `stack/detect.rs` drifted to the `.expect` pattern.

**Why it matters**: DUP-3 flags 3+ copies of a non-trivial control-flow pattern as a refactor candidate. A shared `fn lock_recover<T>(m: &Mutex<T>) -> MutexGuard<'_, T>` (or an extension trait) gives one site for the policy and eliminates the drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce a shared helper (e.g. in a new internal::sync module or as an extension trait) and migrate all four sites
- [ ] #2 Confirm via grep that no cache.lock().unwrap_or_else open-coding remains
<!-- AC:END -->
