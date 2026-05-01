---
id: TASK-0750
title: >-
  FN-9: Production load_config mutates a process-global AtomicUsize for test
  instrumentation
status: Triage
assignee: []
created_date: '2026-05-01 05:52'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/loader.rs:56-72`

**What**: LOAD_CONFIG_CALL_COUNT is a static AtomicUsize mutated unconditionally inside load_config solely so an integration test (TASK-0427) can assert .ops.toml loads once. Counter is incremented in every CLI invocation including release builds.

**Why it matters**: FN-9: explicit dependencies, no implicit state. Hidden global mutable state observed via load_config_call_count() couples production behaviour to a test-only contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Counter is gated behind #[cfg(any(test, feature = "test-support"))] or replaced with a tracing-event-counted assertion
- [ ] #2 TASK-0427 regression test continues to pass under the new gate
- [ ] #3 load_config_call_count/reset_load_config_call_count no longer appear in release-build symbols
<!-- AC:END -->
