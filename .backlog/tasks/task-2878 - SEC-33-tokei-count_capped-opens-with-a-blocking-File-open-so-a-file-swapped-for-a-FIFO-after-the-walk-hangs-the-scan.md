---
id: TASK-2878
title: 'SEC-33: tokei count_capped opens with a blocking File::open, so a file swapped for a FIFO after the walk hangs the scan'
status: Done
assignee: []
created_date: '2026-11-18 22:54'
updated_date: '2026-11-19 17:22'
labels:
  - code-review-rust
  - security
dependencies: []
parent_task_id: 'TASK-2881'
modified_files:
  - extensions/tokei/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs` (`count_capped`, residual documented on `ScanLimits::file_bytes`)

**What**: `count_capped` checks the opened handle's metadata, but the open itself is a plain blocking open. If a regular file is replaced by a FIFO between the walk and the open, the open blocks with no writer, and no deadline check can interrupt it.

**Why it matters**: a hostile or racing tree can hang `ops` LOC/tokei scans until the process is killed. The same class was fixed for ops-git (O_NONBLOCK open + regular-file check on the handle, now `ops_core::text::read_capped_bytes`) and for ops-about machine config reads.

**Origin**: discovered during code-review run 2026-11-18 (wave TASK-2855, member TASK-2819 documented it as a residual rather than fixing it).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 count_capped opens without blocking on a FIFO (O_NONBLOCK or the shared ops_core capped-open helper) and skips non-regular files
- [x] #2 a unix test swaps a candidate for a FIFO and the scan returns promptly

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
count_capped now opens through ops_core::text::open_regular_following (O_NONBLOCK, fstat S_IFREG on the handle, O_NONBLOCK cleared before reading), keeping the encoding_rs_io decoder over take(cap+1) on the returned handle. It follows symlinks rather than using the strict no-symlink walk: tokei's own parse follows them, the scan root is the caller's unresolved working dir (may sit under a symlinked ancestor), and only line counts leave the function. A non-regular file is an Err and lands in skipped_unreadable. ScanLimits::file_bytes residual note removed. A directory now surfaces as IsADirectory (test updated). Test count_capped_refuses_a_candidate_swapped_for_a_fifo walks a tree, swaps the candidate for a FIFO, and asserts a prompt InvalidInput.
<!-- SECTION:NOTES:END -->
