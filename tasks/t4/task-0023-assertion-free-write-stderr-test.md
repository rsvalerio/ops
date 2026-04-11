---
id: TASK-023
title: "Assertion-free test write_stderr_handles_none_and_some in display.rs"
status: Triage
assignee: []
created_date: '2026-04-09 00:00:00'
labels: [rust-test-quality, TQ, TEST-1, medium, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/display.rs:805-808`
**Anchor**: `fn write_stderr_handles_none_and_some`
**Impact**: This test calls `write_stderr(None)` and `write_stderr(Some("test line"))` with no assertions whatsoever. It only verifies the function does not panic, providing false confidence in coverage metrics. A regression that silently discards output or writes corrupted data would pass this test.

**Notes**:
The `write_stderr` function writes to stderr. Testing actual output requires capturing stderr (e.g., via a `Write` trait object or by redirecting output). At minimum, the test should document why it is assertion-free (panic-safety only) or be upgraded to verify output content.
