---
id: TASK-2074
title: 'ERR-4: bare current_dir() in ops init bypasses the crate''s contextual cwd() helper'
status: Done
assignee: []
created_date: '2026-09-07 22:56'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - error-handling
dependencies: []
parent_task_id: 'TASK-2249'
modified_files:
  - crates/cli/src/init_cmd.rs
priority: low
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/init_cmd.rs:23`

**What**: `run_init_to` calls `std::env::current_dir()?` directly with no `.context(...)`, bypassing the shared `crate::cwd()` helper (crates/cli/src/main.rs:356) that exists precisely to attach the "failed to read current working directory" context to this call. Every other subcommand handler routes through `crate::cwd()`; `hook_shared.rs:120` adds an equivalent context on the same operation for the hook-install path.

**Why it matters**: A failing `current_dir()` (deleted cwd, EACCES on a parent component) surfaces from `ops init` as a bare "No such file or directory (os error 2)" with no operation name — exactly the symptom the `cwd()` helper was introduced to eliminate. One handler escaping the shared helper also invites the next handler to copy the bare form.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 init_cmd resolves cwd via crate::cwd() (or an equivalent .context naming the operation), so a failure names the operation
- [x] #2 no other bare std::env::current_dir() call sites remain in crates/cli production code
<!-- AC:END -->
