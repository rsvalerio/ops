---
id: TASK-1614
title: >-
  ERR-7: capture_path_index_from silently swallows per-entry IO errors via
  .flatten()
status: Done
assignee:
  - TASK-1637
created_date: '2026-05-22 06:51'
updated_date: '2026-05-22 12:53'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe/path.rs:58`

**What**: `for entry in entries.flatten() { ... }` in `capture_path_index_from` discards every per-entry `io::Error` (transient EIO on a network mount, EACCES on a partially-restricted dir, ENOENT during dir mutation). The outer `read_dir` failure is logged via `tracing::warn!`, but per-entry iterator errors vanish silently — affected basenames simply do not enter the index, so tools that genuinely live on `$PATH` get probed as `NotInstalled` and re-installed.

**Why it matters**: This is the same class of bug the surrounding `read_dir` arm goes out of its way to log (with the ERR-7 / TASK-1563 Debug-escape treatment). The `.flatten()` shortcut undermines that contract. Pair-program with the broader PathIndex regression to `NotInstalled` → reinstall described in API / TASK-1200's policy commentary.

<!-- scan confidence: high; one call site -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace .flatten() with explicit Ok/Err match that emits a tracing::warn! (Debug-rendered path) on per-entry errors, mirroring the existing read_dir warn path.
- [x] #2 Add a unit test on Unix that builds a PATH dir containing one unreadable entry (e.g. dangling symlink under a no-exec dir) and asserts the warn fires while other entries still index.
<!-- AC:END -->
