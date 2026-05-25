---
id: TASK-1134
title: 'ERR-1: atomic_write leaves orphaned tmp files on write/sync failure'
status: Done
assignee:
  - TASK-1268
created_date: '2026-05-08 07:40'
updated_date: '2026-05-09 17:30'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:176`

**What**: When rename fails, cleanup runs `remove_file(&tmp)`. But `f.write_all` / `f.sync_all` errors propagate via `?` (lines 172-173) and leave the temp file on disk with no cleanup.

**Why it matters**: A series of partial writes against a target on a flaky disk leaves orphaned `.foo.tmp.<pid>.<counter>.<nanos>` files. The atomic-write contract documents temp cleanup on rename failure but not on write/sync failure — the more common mode under disk pressure.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 On f.write_all / f.sync_all failure inside the inner block at lines 150-174, remove_file(&tmp) and warn on cleanup failure
- [x] #2 Add a regression test that injects a write failure and asserts no .tmp. leftovers persist
<!-- AC:END -->
