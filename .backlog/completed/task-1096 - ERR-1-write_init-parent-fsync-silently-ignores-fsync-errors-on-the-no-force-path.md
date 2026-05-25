---
id: TASK-1096
title: >-
  ERR-1: write_init parent fsync silently ignores fsync errors on the no-force
  path
status: Done
assignee: []
created_date: '2026-05-07 21:32'
updated_date: '2026-05-07 23:18'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/init_cmd.rs:76-87`

**What**: The Unix parent-fsync block uses `let _ = dir.sync_all()` — a failing parent fsync (ENOSPC, EIO) returns silently while the function reports success. The parent open failure (`fs::File::open(parent).is_err()`) also silently skips the fsync. Compare to the symmetric `atomic_write` path which now warns at TASK-0899 / ERR-1 in `edit.rs:166-180`.

**Why it matters**: Asymmetry with atomic_write was the very pattern TASK-0730 set out to close; init_cmd regressed it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Parent fsync failure on the no-force path emits a tracing::warn! with the parent path and error, matching edit::atomic_write
- [ ] #2 The parent open failure also warns instead of silently skipping the fsync
- [ ] #3 Existing crash-safety semantics preserved (warn, not error — the rename has succeeded)
<!-- AC:END -->
