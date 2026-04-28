---
id: TASK-0425
title: >-
  DUP-1: cli init_cmd::write_init reimplements atomic-write without parent-dir
  fsync
status: To Do
assignee:
  - TASK-0536
created_date: '2026-04-28 04:41'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/init_cmd.rs:61-93`

**What**: The `force` branch builds its own `.{file}.tmp.{pid}.{nanos}` temp + rename, mirroring `ops_core::config::edit::atomic_write` (which is fn-private, not exposed). Unlike the canonical helper, the local copy omits the post-rename parent-directory fsync, so on Linux ext4 a crash after rename can lose the new dirent (the same hardening rationale documented at `crates/core/src/config/edit.rs:111-117`).

**Why it matters**: The two implementations will drift; the cli copy already silently drops the durability behaviour the core copy went out of its way to add, undermining the SEC-32 / SEC-25 guarantees that .ops.toml writes are supposed to provide.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Promote atomic_write to pub(crate) or pub in ops_core::config and have init_cmd::write_init (force branch) delegate to it
- [ ] #2 Remove the duplicated temp-name + rename logic from init_cmd.rs
- [ ] #3 Non-force branch (create_new) keeps its current behaviour but its sync_all happy-path stays
<!-- AC:END -->
