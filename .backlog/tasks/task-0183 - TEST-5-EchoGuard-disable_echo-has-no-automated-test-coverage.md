---
id: TASK-0183
title: 'TEST-5: EchoGuard::disable_echo has no automated test coverage'
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - TEST
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/terminal.rs:20-83`

**What**: `EchoGuard::disable_echo` and its `Drop` impl contain four `unsafe` blocks calling `libc::tcgetattr` / `libc::tcsetattr`. The docstring explicitly states "Tested manually; unit tests would require a PTY or mock which exceeds the complexity budget for ~60 lines of platform-specific code." With zero tests, a regression in the termios flag mask (e.g. accidentally clearing more than ECHO, forgetting to restore in Drop, or mis-handling the non-TTY branch) will ship unnoticed.

**Why it matters**: TEST-5 (public API must have at least one test). Even without a full PTY harness, the non-TTY early-return branch is trivially testable (set stderr to a pipe, assert `original == None`). A `pty-rs` / `nix::pty::openpty` test would cover the happy path on Unix. Recommended: at minimum add the non-TTY unit test now, file a follow-up for the PTY-based Drop test. Related to UNSAFE-1 in TASK-0140.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a test that verifies disable_echo returns a no-op guard when stderr is not a TTY
- [ ] #2 Evaluate feasibility of a nix::pty-based test that asserts ECHO is cleared and restored
<!-- AC:END -->
