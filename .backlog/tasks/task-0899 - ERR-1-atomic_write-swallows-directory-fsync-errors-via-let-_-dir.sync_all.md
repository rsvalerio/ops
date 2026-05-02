---
id: TASK-0899
title: 'ERR-1: atomic_write swallows directory fsync errors via let _ = dir.sync_all()'
status: Done
assignee: []
created_date: '2026-05-02 10:08'
updated_date: '2026-05-02 11:15'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:121`

**What**: After rename succeeds, the parent-directory fsync result is discarded with `let _ = dir.sync_all()`. A failing fsync (ENOSPC, EIO, full disk) is silently ignored, defeating the crash-safety guarantee documented just above.

**Why it matters**: On Linux ext4 the directory fsync is the difference between the rename surviving and not surviving a power loss; failing it silently hides the only signal that crash-safety is currently broken.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Log a tracing::warn! when dir.sync_all() returns Err, including the parent path and the io::Error
- [ ] #2 Document that the fsync error is non-fatal but visible at warn level
<!-- AC:END -->
