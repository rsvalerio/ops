---
id: TASK-003
title: "EchoGuard unsafe FFI code has no tests"
status: To Do
assignee: []
created_date: '2026-04-06 00:00:00'
labels: [rust-test-quality, TQ, TEST-5, medium, effort-M, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/terminal.rs:1-79`
**Anchor**: `fn disable_echo`, `impl Drop for EchoGuard`
**Impact**: `EchoGuard` contains unsafe FFI calls to `libc::tcgetattr` and `libc::tcsetattr` with no test coverage. The code is used in production (`run_cmd.rs:58`, `run_cmd.rs:222`), but in test environments stderr is not a TTY, so the unsafe path is never exercised — only the early-return no-op path runs.

**Notes**:
The code is well-designed defensively (SAFETY comments, error-checked returns, RAII wrapper), so the risk is mitigated. However, the unsafe paths remain unverified.

To test, create a PTY-based test that:
1. Allocates a pseudo-terminal (`nix::pty::openpty` or `portable-pty` crate)
2. Redirects stderr to the PTY slave
3. Calls `EchoGuard::disable_echo()` and verifies echo is disabled (via `tcgetattr` check)
4. Drops the guard and verifies echo is restored

Alternative: use `libc::tcgetattr` directly in a test to verify the guard modifies and restores the `ECHO` flag on a real TTY fd, if the test environment provides one (mark `#[ignore = "requires TTY"]` for CI).
