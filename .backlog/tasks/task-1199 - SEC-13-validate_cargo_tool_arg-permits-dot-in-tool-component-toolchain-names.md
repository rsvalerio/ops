---
id: TASK-1199
title: 'SEC-13: validate_cargo_tool_arg permits dot in tool/component/toolchain names'
status: Done
assignee:
  - TASK-1260
created_date: '2026-05-08 08:14'
updated_date: '2026-05-08 14:08'
labels:
  - code-review-rust
  - sec
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/install.rs:23-45`

**What**: validate_cargo_tool_arg accepts `[A-Za-z0-9][A-Za-z0-9_.\-]*`, including `.`. crates.io's grammar is `[a-zA-Z][a-zA-Z0-9_-]*` (no dot), and rustup component / toolchain names follow the same shape. Allowing `.` lets entries like tool.cargo or cargo.deny.something pass the SEC-13 defense-in-depth check and reach cargo install / rustup component add.

**Why it matters**: The validator's purpose is conservative defence-in-depth. A grammar that admits `.` invites future contributors to assume the validator has vetted names and trust them in path-construction or display contexts where `.` carries meaning.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 validate_cargo_tool_arg('ops.bad', 'tool name') returns Err whose message names the offending . character.
- [x] #2 All existing call sites with legitimate names (cargo-deny, cargo-edit, clippy, rustfmt, rust-src) continue to validate.
<!-- AC:END -->
