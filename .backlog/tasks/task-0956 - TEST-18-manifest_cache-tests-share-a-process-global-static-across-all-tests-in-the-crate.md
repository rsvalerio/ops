---
id: TASK-0956
title: >-
  TEST-18: manifest_cache tests share a process-global static across all tests
  in the crate
status: Triage
assignee: []
created_date: '2026-05-04 21:46'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/manifest_cache.rs:50,119-141,153-174` and `extensions-node/about/src/manifest_cache.rs:25,89-108,119-135`

**What**: `static CACHE: OnceLock<Mutex<HashMap<...>>>` is shared across every test in the crate's test binary. The `poison_recovery_keeps_cache_usable` test deliberately poisons the mutex and never un-poisons it; subsequent tests in the same binary see a permanently-poisoned mutex and exercise the fallback path. Order-dependent assertions (e.g. count `>= 3` in `arc_is_shared_across_two_consumer_parses`) are fragile against shared state.

**Why it matters**: TEST-18 forbids shared mutable fixtures without isolation; TEST-23 requires cleanup of shared state between runs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Refactor cache to be injectable (carry through Context) so tests can use a fresh instance, OR add a #[cfg(test)] cache-clear helper invoked at the start of each cache test
- [ ] #2 poison_recovery_* tests are isolated (dedicated test binary or serial_test) so they cannot bleed into other tests
<!-- AC:END -->
