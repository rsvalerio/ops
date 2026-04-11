---
id: TASK-033
title: Weak assertions in runner command tests check only is_ok/is_err
status: To Do
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-11, low, effort-S, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/command/tests.rs:345-350, 505-511`
**Anchor**: `fn run_unknown_command_returns_error`, `fn execute_with_timeout_with_timeout_returns_output`
**Impact**: Two tests use bare `is_err()` / `is_ok()` assertions without verifying the actual result value, weakening their ability to catch regressions.

**Notes**:
`run_unknown_command_returns_error` (line 349): `assert!(result.is_err())` should also verify the error message contains a meaningful substring (e.g., `"nonexistent"` or `"not found"`), matching the pattern used in similar tests at lines 1395-1408 which correctly check `result.unwrap_err().to_string().contains(...)`.

`execute_with_timeout_with_timeout_returns_output` (line 510): `assert!(result.is_ok())` is the only assertion — it doesn't verify the output at all. Compare with `execute_with_timeout_no_timeout_succeeds` (line 500-502) which correctly unwraps and checks `output.status.success()`. Apply the same pattern here.
