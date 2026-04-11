---
id: TASK-005
title: "No tests for runner/terminal.rs"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-5, low, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/terminal.rs`
**Anchor**: (entire file)
**Impact**: Terminal echo control utilities have no tests. While this is low-level terminal I/O that may be difficult to unit-test, the gap should be documented.

**Notes**:
- TEST-5: All public API functions must have at least one test
- This file provides terminal echo enable/disable for password-style input
- If the API is testable (e.g., verifying state transitions or error handling), add tests; otherwise document the gap with a comment
- Severity is low because this is platform-specific I/O code where mocking is non-trivial
