---
id: TASK-1425
title: >-
  PERF-3: read_capped does not pre-size buf; multi-MiB captures pay O(log N)
  doubling reallocations
status: Done
assignee:
  - TASK-1450
created_date: '2026-05-13 18:22'
updated_date: '2026-05-13 19:15'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:212`

**What**: `read_capped` reads into a stack chunk then `extend_from_slice` into `buf` without pre-allocating. For multi-MiB captures (cargo metadata, large stdout) the Vec doubles repeatedly from empty.

**Why it matters**: Subprocess output capture is on the every-command hot path. A `Vec::with_capacity(min(cap, 64 * 1024))` would amortise allocations.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Pre-size buf with an initial capacity bounded by cap
- [ ] #2 Verify allocation count drops for >1 MiB captures (bench or counter)
<!-- AC:END -->
