---
id: TASK-0013
title: EchoGuard in terminal.rs has no test coverage
status: Done
assignee: []
created_date: '2026-04-10 18:00:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-test-quality
  - TQ
  - TEST-5
  - low
  - crate-runner
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/runner/src/terminal.rs:16-79`
**Anchor**: `struct EchoGuard`, `fn disable_echo`, `fn drop`
**Impact**: `EchoGuard` is a public struct with `disable_echo()` constructor and `Drop` impl that restores terminal state. It has zero test coverage — no unit tests, no integration tests. The struct manipulates terminal attributes via `libc::tcgetattr`/`libc::tcsetattr`, which is platform-specific and legitimately hard to test in CI (no real TTY).

**Notes**:
- Testing the full termios path requires a PTY or mock, which may not be worth the complexity for ~60 lines of platform-specific code.
- A pragmatic middle ground: test the struct's state machine (original attrs stored, restored on drop) by mocking the termios calls behind a trait boundary. This would require a small refactor.
- Alternatively, mark this as an accepted gap with a `// TEST-5: platform-specific terminal control, tested manually` comment.
- The `Drop` impl logs errors via `tracing::debug` on failure — at minimum a no-panic test with a non-TTY fd could verify graceful degradation.
<!-- SECTION:DESCRIPTION:END -->
