---
id: TASK-0130
title: 'ERR-10: resolve_exec_leaf returns Result<ExecCommandSpec, String>'
status: To Do
assignee: []
created_date: '2026-04-22 21:15'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - err
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:313`

**What**: `resolve_exec_leaf` returns `Result<ExecCommandSpec, String>`, building error messages via `format!` rather than a typed error enum.

**Why it matters**: String errors lose structure; callers cannot match on failure kind (unknown command vs. composite-in-leaf) to render diagnostics or recover. Library API should use a domain error enum.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce a typed error enum (e.g., ResolveExecError) with variants for UnknownCommand and CompositeInLeafPlan
- [ ] #2 Update resolve_exec_leaf signature and all callers to use the new error type
<!-- AC:END -->
