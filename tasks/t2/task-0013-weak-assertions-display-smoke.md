---
id: TASK-0013
title: "Useless: display.rs smoke tests have zero assertions"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-1, low, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/display.rs:521-end`
**Anchor**: `fn emit_line_handles_empty_string`, `fn handle_event_unknown_command_id_no_panic`, `fn finish_step_unknown_id_returns_none`, `fn handle_event_step_failed_for_unknown_command_no_panic`, `fn write_stderr_handles_none_and_some`
**Impact**: Five tests have zero assertions — they only verify no-panic behavior. While panic prevention has value, these tests create false coverage metrics without verifying any observable behavior.

**Notes**:
These are intentionally smoke tests for edge cases (empty strings, unknown IDs). The no-panic property is valuable but could be documented explicitly with comments. Low severity because these are edge-case paths, not core functionality. Consider adding at minimum a state assertion (e.g., after `emit_line("")`, verify the display state is unchanged).
