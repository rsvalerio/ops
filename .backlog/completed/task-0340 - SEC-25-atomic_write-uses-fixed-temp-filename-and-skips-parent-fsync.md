---
id: TASK-0340
title: 'SEC-25: atomic_write uses fixed temp filename and skips parent fsync'
status: Done
assignee:
  - TASK-0416
created_date: '2026-04-26 09:34'
updated_date: '2026-04-27 08:26'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:70-94`

**What**: atomic_write builds a sibling temp at a deterministic name .filename.tmp. Two concurrent ops invocations race on the same temp path: one process can File::create and overwrite the other in-progress write. After rename(), the parent directory is not fsync-d.

**Why it matters**: The module contract is "a crash mid-write leaves the previous content intact". The deterministic temp + missing dir-sync weakens that guarantee.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Generate a unique temp suffix (e.g. tempfile::NamedTempFile::new_in or PID+nanos) so concurrent writers cannot collide
- [x] #2 After rename, open the parent directory and call sync_all (Unix) so the directory entry is durable
<!-- AC:END -->
