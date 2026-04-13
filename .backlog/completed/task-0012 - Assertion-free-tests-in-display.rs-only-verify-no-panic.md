---
id: TASK-0012
title: Assertion-free tests in display.rs only verify no-panic
status: Done
assignee: []
created_date: '2026-04-10 18:00:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-test-quality
  - TQ
  - TEST-1
  - low
  - crate-runner
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/runner/src/display.rs:694-697`, `crates/runner/src/display.rs:820-823`
**Anchor**: `fn emit_line_handles_empty_string`, `fn write_stderr_handles_none_and_some`
**Impact**: Two tests have no `assert!` / `assert_eq!` statements — they call functions and rely solely on not panicking. While no-panic verification has value, these tests create false confidence in coverage metrics without verifying behavior.

**Notes**:
- `emit_line_handles_empty_string` (line 694): calls `display.emit_line("")` with no assertions. Could at minimum assert the display state is unchanged, or capture stderr output.
- `write_stderr_handles_none_and_some` (line 820): calls `write_stderr(None)` and `write_stderr(Some("test line"))` with no assertions. Could verify that output was written (e.g., by redirecting stderr or checking return values).
- If no-panic is the genuine intent, add a `// verifies no-panic on edge input` comment per TEST-1 convention, or add lightweight assertions on observable side effects.
<!-- SECTION:DESCRIPTION:END -->
