---
id: TASK-1086
title: >-
  SEC-25: atomic_write mode preservation masked by umask, can silently widen
  file ACLs
status: Done
assignee: []
created_date: '2026-05-07 21:31'
updated_date: '2026-05-08 12:00'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:134-149`

**What**: `OpenOptions::mode(mode).create_new(true)` does NOT bypass the process umask — POSIX `open(2)` creates the file with `mode & ~umask`. A destination at `0o600` and umask `022` happens to round-trip, but `0o660` group-writable destinations collapse to `0o640`, and any future caller writing under a non-default umask will silently widen permissions across the rename. TASK-0898 only landed the `.mode()` request side; the umask hole remains.

**Why it matters**: Permission widening on config writes is a defensive-posture regression. The fix is to `fchmod` the temp fd after creation (or `set_permissions` on the temp path before `rename`) so the requested mode is exact regardless of umask.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 After temp creation, the actual on-disk mode equals the destination's mode (verified via stat) regardless of process umask
- [x] #2 A regression test sets umask to 0o077, writes via atomic_write over a 0o644 destination, and asserts post-write mode is exactly 0o644
- [x] #3 The fix preserves no-op behaviour on non-Unix targets
<!-- AC:END -->
