---
id: TASK-1461
title: >-
  SEC-25: TOCTOU between symlink_metadata probe and File::open in
  read_capped_to_string_with
status: Done
assignee:
  - TASK-1478
created_date: '2026-05-15 18:50'
updated_date: '2026-05-17 07:18'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:141-160`

**What**: `read_capped_to_string_with` probes the path with `std::fs::symlink_metadata` and bails if it is a symlink, then calls `std::fs::File::open` (which follows symlinks). The two-syscall pattern is a textbook TOCTOU race: between the probe and the open, an adversarial process can swap a regular file for `package.json -> /etc/passwd`, defeating the SEC-25 guard the module comment claims.

**Why it matters**: `ops` is invoked on third-party repos and the whole motivation for the symlink guard (TASK-1442) is to prevent privileged-file disclosure through manifest readers. A scheduling-window race defeats the guard with no warning. Fix is to atomically refuse symlinks at the kernel level via `O_NOFOLLOW`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_capped_to_string_with opens the file with O_NOFOLLOW (Unix) and the platform equivalent on non-Unix, removing the symlink_metadata pre-probe
- [ ] #2 Regression test plants a symlink at the manifest path concurrently and asserts the open returns an error mapped from ELOOP (or InvalidInput) without ever reading the link target
<!-- AC:END -->
