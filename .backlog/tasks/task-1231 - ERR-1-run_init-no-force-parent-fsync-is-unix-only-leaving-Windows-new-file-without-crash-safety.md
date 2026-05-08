---
id: TASK-1231
title: >-
  ERR-1: run_init no-force parent fsync is unix-only, leaving Windows new file
  without crash safety
status: To Do
assignee:
  - TASK-1268
created_date: '2026-05-08 12:58'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/init_cmd.rs:81-113`

**What**: `write_init`'s no-force branch performs the parent-directory fsync only inside `#[cfg(unix)]`. On Windows the `create_new` branch returns success without any equivalent durability call and without a tracing::warn! noting the gap; the --force branch goes through `atomic_write` which itself only fsyncs on Unix.

**Why it matters**: A power loss after `ops init` on Windows can lose the new .ops.toml directory entry. Silent platform asymmetry — operators reading the unix branch's "may not survive a power loss" warn assume the same applies on Windows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a Windows fsync analogue (FlushFileBuffers / MOVEFILE_WRITE_THROUGH)
- [ ] #2 OR document the platform gap with a tracing::debug! breadcrumb on Windows
- [ ] #3 Mirror the durability policy across both force and no-force branches
<!-- AC:END -->
