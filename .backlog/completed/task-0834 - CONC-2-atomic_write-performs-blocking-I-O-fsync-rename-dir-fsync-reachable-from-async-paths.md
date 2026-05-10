---
id: TASK-0834
title: >-
  CONC-2: atomic_write performs blocking I/O (fsync/rename/dir-fsync) reachable
  from async paths
status: Done
assignee: []
created_date: '2026-05-02 09:12'
updated_date: '2026-05-02 12:22'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:75-125`

**What**: atomic_write performs blocking I/O - write, sync_all, rename, parent-dir sync_all - on the calling thread. The module is the canonical edit helper for CLI handlers; some callers run inside a Tokio runtime.

**Why it matters**: fsync can stall a thread for tens to hundreds of milliseconds on slow disks, freezing the runtime. CONC-5 mandates tokio::fs or spawn_blocking for fs I/O on async paths.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add doc note that callers in async contexts must wrap in tokio::task::spawn_blocking (mirroring subprocess.rs:14-23 style)
- [ ] #2 Or expose an async sibling atomic_write_async using tokio::fs + spawn_blocking for the dir-fsync
- [x] #3 Audit current call sites under async fns and migrate them
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added doc note on atomic_write per AC #1 (mirrors subprocess.rs Sync-only contract). Audit (AC #3) found no async call sites: callers in crates/cli/* (about_cmd, new_command_cmd, theme_cmd, init_cmd) and extensions/{hook-common,duckdb} are all sync. AC #2 not needed.
<!-- SECTION:NOTES:END -->
