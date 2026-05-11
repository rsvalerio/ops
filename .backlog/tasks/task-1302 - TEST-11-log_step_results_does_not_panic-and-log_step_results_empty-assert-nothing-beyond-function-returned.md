---
id: TASK-1302
title: >-
  TEST-11: log_step_results_does_not_panic and log_step_results_empty assert
  nothing beyond 'function returned'
status: To Do
assignee:
  - TASK-1305
created_date: '2026-05-11 16:36'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/tests.rs:336-349`

**What**: Two tests call `log_step_results(...)` and `log_step_results(&[])` respectively without any assertion at all — the test name `log_step_results_does_not_panic` is the only contract, and Rust tests already fail on panic, so the body adds zero signal beyond "this function exists and we can call it". `log_step_results` is supposed to emit one `tracing::debug!` event per `StepResult`; neither test installs a subscriber to capture the events and verify the fields (id, success, duration_ms, stdout_len, stderr_len, message) actually get logged.

**Why it matters**: Similar in spirit to TASK-1299 (other tests in this file that only assert is_ok/is_err) but for a function whose entire contract is its emitted log fields. A regression that swaps two field bindings, drops the success flag, or stops iterating after the first result will pass these tests green. Either capture tracing output via a `MakeWriter` subscriber (the file already uses this pattern in extension_summary_warns_on_self_shadow and other registry tests) and assert the per-result fields, or delete the tests as they document a property the type system already enforces.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 log_step_results_does_not_panic either installs a tracing subscriber to capture and assert the emitted fields, or is removed as redundant
- [ ] #2 log_step_results_empty either asserts no debug events are emitted (via a captured subscriber) or is removed
- [ ] #3 If kept, the tests follow the BufWriter + MakeWriter pattern already used in registry/tests.rs for tracing capture
<!-- AC:END -->
