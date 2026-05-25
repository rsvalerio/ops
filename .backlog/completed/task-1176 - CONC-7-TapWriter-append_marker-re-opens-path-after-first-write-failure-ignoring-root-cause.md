---
id: TASK-1176
title: >-
  CONC-7: TapWriter::append_marker re-opens path after first write failure
  ignoring root cause
status: Done
assignee:
  - TASK-1261
created_date: '2026-05-08 08:08'
updated_date: '2026-05-08 14:47'
labels:
  - code-review-rust
  - conc
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/tap.rs:89`

**What**: After `write_line` records a truncation (typically due to ENOSPC or EPIPE) and drops the file handle, `append_marker` re-opens the same path with `OpenOptions::append`. If the failure was disk-full, the re-open succeeds but the marker write fails again; if the path is on a now-stale NFS mount, both operations may hang. The function silently retries the failed I/O while claiming "best effort".

**Why it matters**: A genuine ENOSPC condition produces two consecutive log lines per truncation (open-ok, write-fail). On truncation paths backed by a hung mount this re-issues blocking I/O on a synchronous code path that is already running on the display thread.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 append_marker short-circuits when the prior failure kind is StorageFull / BrokenPipe (or skips when the file handle was already lost to a write error of the same kind).
- [ ] #2 The hung-mount case is bounded by either non-blocking reopen, or the re-open is done on the blocking pool with a short timeout.
<!-- AC:END -->
