---
id: TASK-1158
title: 'ARCH-1: extensions-rust/tools/src/probe.rs is 677 lines mixing five concerns'
status: To Do
assignee:
  - TASK-1264
created_date: '2026-05-08 07:44'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:1`

**What**: Single 677-line module mixes: (1) timeout-aware probe wrapper, (2) `rustup show active-toolchain` parsing + diagnostic-prefix rejection, (3) `cargo --list` scanning + 39-entry CARGO_BUILTIN_SUBCOMMANDS allowlist, (4) $PATH walking with platform-specific PATHEXT/exec-bit handling and PathIndex cache, (5) rustup-component listing + 30-entry RUSTUP_TARGET_ARCH_PATTERNS triple-stripping.

**Why it matters**: Editing the rustup-component path forces re-reading PATH-walk and cargo-list logic. Two large &[&str] constants maintained inline rather than as data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Split into probe/path.rs, probe/cargo.rs, probe/rustup.rs, probe/timeout.rs leaving probe/mod.rs as a ≤100 line dispatcher
- [ ] #2 Each submodule ≤300 lines
- [ ] #3 check_tool_status_with stays the public composition point
<!-- AC:END -->
