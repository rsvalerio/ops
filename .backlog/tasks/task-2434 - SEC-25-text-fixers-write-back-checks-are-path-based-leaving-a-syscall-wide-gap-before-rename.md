---
id: TASK-2434
title: 'SEC-25: text-fixers write-back checks are path-based, leaving a syscall-wide gap before rename'
status: Triage
assignee: []
created_date: '2026-10-04 15:55'
labels:
  - code-review-rust
  - SEC
dependencies: []
modified_files:
  - extensions/text-fixers/src/atomic.rs
  - crates/core/src/text.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/text-fixers/src/atomic.rs` (`replace`, `refuse_symlinked_directory`, `ensure_unchanged`)

**What**: `replace` now refuses a symlinked directory component and re-`lstat`s the target (dev/ino/len/mtime) right before `persist`, but both checks resolve the path by name, and so do the stage-file creation and the `rename(2)`. A directory swapped in between a check and the syscall it guards, or an edit landing between the `lstat` and the rename, is still not seen. A same-length in-place edit that also restores the mtime is not detected either (ctime is not compared). Closing the gap needs the write-back to run relative to a directory handle (`openat` the parent with `O_NOFOLLOW` per component, stage with `openat`, `fstatat` the target, `renameat`), which needs FFI that `ops-text-fixers` cannot hold under `unsafe_code = "deny"`; the natural home is next to `ops_core::text::open_refusing_symlinks`.

**Why it matters**: The read half is handle-based and race-free; the write half only narrows the window. The module docs state the limit, so behaviour is documented, but the structural SEC-25 fix (acquire a handle, then act through it) is not in place for the rename.

**Origin**: discovered during TASK-2422 while fixing TASK-2401.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 atomic::replace stages and renames relative to a parent-directory handle opened without following symlinks, or the residual window is recorded as an accepted limit in an ADR
- [ ] #2 The changed-since-read comparison also covers ctime on Unix, or the docs say why it does not
<!-- AC:END -->
