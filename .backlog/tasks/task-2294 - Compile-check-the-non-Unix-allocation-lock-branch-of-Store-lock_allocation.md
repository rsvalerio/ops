---
id: TASK-2294
title: 'Compile-check the non-Unix allocation-lock branch of Store::lock_allocation'
status: To Do
assignee: []
created_date: '2026-09-26 19:03'
updated_date: '2026-09-26 19:55'
labels:
  - code-review-rust
  - portability
dependencies: []
parent_task_id: 'TASK-2305'
modified_files:
  - crates/backlog/src/store.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/store.rs` (`Store::lock_allocation`)

**What**: `task create --unless-exists` serializes its check-then-create through `Store::lock_allocation`. On Unix it locks the `tasks/` directory fd; the `#[cfg(not(unix))]` branch opens a persistent `.allocation.lock` file in the backlog root. No gate in this workspace (Linux `ops verify`/`qa`) compiles or tests that branch, while release builds may target Windows.

**Why it matters**: a compile error or behavioral bug there (e.g. the lock file being picked up as untracked noise, or the lock not excluding concurrent creates) would only surface on a Windows build.

**Origin**: discovered during TASK-2293 while fixing TASK-2284.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The non-Unix branch is compiled by a gate (e.g. cargo check --target x86_64-pc-windows-gnu or a Windows CI job)
- [ ] #2 The concurrent keyed-create test passes on that platform, or the gap is documented
<!-- AC:END -->
