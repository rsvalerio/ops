---
id: TASK-007
title: "resolve_exec_specs uses String (CommandId alias) as error type"
status: Triage
assignee: []
created_date: '2026-04-09 19:10:00'
labels: [rust-idioms, EFF, ERR-10, low, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/command/mod.rs:340-354`
**Anchor**: `fn resolve_exec_specs`
**Impact**: The function signature `Result<Vec<(CommandId, ExecCommandSpec)>, CommandId>` expands to `Result<..., String>` since `CommandId` is a type alias for `String`. This technically violates ERR-10 ("Never use `Result<T, String>`—use proper error types"). While the alias gives semantic meaning and the method is private with a single call site, a lightweight error type would make the failure mode self-documenting and prevent accidental misuse if the function is refactored or exposed later.

**Notes**:
The single caller (line 456) uses the returned `CommandId` only to build a format string: `format!("internal error: composite in leaf plan: {}", id)`. A simple newtype `struct UnresolvedCommand(CommandId)` or an enum variant on an existing error type would satisfy ERR-10 without adding complexity. Severity is Low because the method is private, the alias provides clear intent, and the caller handles the error correctly.
