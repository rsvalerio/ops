---
id: TASK-1464
title: 'ERR-1: sync_parent_dir silently swallows parent-directory open failure on Unix'
status: To Do
assignee:
  - TASK-1482
created_date: '2026-05-15 18:50'
updated_date: '2026-05-17 07:06'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:241-251`

**What**: `sync_parent_dir` warns when `dir.sync_all()` fails but is silent when `std::fs::File::open(parent)` itself fails. An EACCES (or any other error) on the parent directory open returns silently with no `tracing::warn!` and no debug breadcrumb.

**Why it matters**: `atomic_write` documents a crash-safety contract that depends on the parent-directory fsync. Silently skipping the fsync because the directory could not be opened means the user believes their write is durable when it is not, and there is no observability breadcrumb to diagnose subsequent corruption after a power loss.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 sync_parent_dir emits a tracing::warn! (matching the sync_all arm) when File::open(parent) fails on Unix, naming the path and error
- [ ] #2 Test with an unreadable parent directory exercises the new branch and asserts the warn fires
<!-- AC:END -->
