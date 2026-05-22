---
id: TASK-1586
title: 'API-5: public pure functions in tools crate lack #[must_use]'
status: Done
assignee:
  - TASK-1638
created_date: '2026-05-21 22:45'
updated_date: '2026-05-22 13:17'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/lib.rs:125,149,158` and `extensions-rust/tools/src/probe/path.rs:88,96` and `extensions-rust/tools/src/probe/cargo.rs:85` and `extensions-rust/tools/src/probe/rustup.rs:8,143`

**What**: Several public pure / side-effect-free query functions are not annotated `#[must_use]`:
- `lib.rs:125` `collect_tools_with`
- `lib.rs:149` `collect_tool_one` (already has `#[must_use]`)
- `lib.rs:158` `collect_tools`
- `probe/path.rs:88` `check_binary_installed_with`
- `probe/path.rs:96` `check_binary_installed`
- `probe/cargo.rs:85` `capture_cargo_list`
- `probe/rustup.rs:8` `get_active_toolchain`
- `probe/rustup.rs:143` `capture_rustup_components`

(`collect_tool_one` is already annotated — apply the same treatment to its peers.) `ToolInfo::new` and `ToolStatus`/`ToolInfo` types themselves also benefit but are constructors so the value is generally bound.

**Why it matters**: API-5 — `#[must_use]` on query functions that return data the caller almost certainly needs catches `let _ = check_binary_installed(...)` and similar bugs at compile time. The probe functions in particular have no side effects beyond a subprocess spawn; ignoring the return value is always a bug.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 all listed public functions carry #[must_use] (or a justification comment if explicitly skipped)
- [x] #2 ProbeOutcome / ToolStatus / ToolInfo / PathIndex types do not need annotation changes
<!-- AC:END -->
