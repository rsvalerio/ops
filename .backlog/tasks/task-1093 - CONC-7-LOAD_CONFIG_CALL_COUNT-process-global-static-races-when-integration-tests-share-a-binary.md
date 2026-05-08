---
id: TASK-1093
title: >-
  CONC-7: LOAD_CONFIG_CALL_COUNT process-global static races when integration
  tests share a binary
status: Done
assignee: []
created_date: '2026-05-07 21:32'
updated_date: '2026-05-08 06:35'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:115-128`

**What**: The `AtomicUsize` is process-global and only reset by the explicit `reset_load_config_call_count()` helper. Two parallel integration tests that both assert "load_config was called once" will race — one test's `fetch_add` increments the other's snapshot. The helper is `pub` so callers can already reach the global state.

**Why it matters**: Flaky tests under cargo's default parallel execution; gated only by convention/discipline, not by typed constraint.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either the counter is gated to a serial-test guard via doc/lint, or it is replaced by a thread-local / per-test instance
- [x] #2 All call sites of load_config_call_count use #[serial_test::serial] (verified via grep)
- [x] #3 Document the global-state hazard in the function rustdoc
<!-- AC:END -->
