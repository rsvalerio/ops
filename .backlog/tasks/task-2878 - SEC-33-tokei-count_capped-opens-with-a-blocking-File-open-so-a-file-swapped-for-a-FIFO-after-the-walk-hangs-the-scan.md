---
id: TASK-2878
title: 'SEC-33: tokei count_capped opens with a blocking File::open, so a file swapped for a FIFO after the walk hangs the scan'
status: Triage
assignee: []
created_date: '2026-11-18 22:54'
labels:
  - code-review-rust
  - security
dependencies: []
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
- [ ] #1 count_capped opens without blocking on a FIFO (O_NONBLOCK or the shared ops_core capped-open helper) and skips non-regular files
- [ ] #2 a unix test swaps a candidate for a FIFO and the scan returns promptly
<!-- AC:END -->
