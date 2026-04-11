---
id: TASK-0004
title: "Test gap: terminal.rs EchoGuard entirely untested"
status: Triage
assignee: []
created_date: '2026-04-08 00:00:00'
labels: [rust-test-quality, TQ, TEST-5, medium, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/terminal.rs:1-end`
**Anchor**: `struct EchoGuard`
**Impact**: The entire `terminal.rs` module has zero test coverage. `EchoGuard::disable_echo()`, its `Drop` impl, the non-TTY no-op path, and the non-Unix no-op path are all untested.

**Notes**:
The TTY-dependent paths (unix `tcgetattr`/`tcsetattr`) are difficult to test in CI. However, the non-TTY path (returns a no-op guard when stderr is not a terminal) and the `#[cfg(not(unix))]` no-op path are trivially testable with unit tests. At minimum, add tests for: (1) non-TTY path returns without error, (2) `EchoGuard` Drop doesn't panic when constructed in no-op mode.
