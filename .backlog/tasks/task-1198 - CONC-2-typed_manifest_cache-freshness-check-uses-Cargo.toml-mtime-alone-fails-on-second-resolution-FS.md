---
id: TASK-1198
title: >-
  CONC-2: typed_manifest_cache freshness check uses Cargo.toml mtime alone,
  fails on second-resolution FS
status: Done
assignee:
  - TASK-1261
created_date: '2026-05-08 08:14'
updated_date: '2026-05-08 14:54'
labels:
  - code-review-rust
  - conc
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:89-93,218-234`

**What**: load_workspace_manifest decides whether to serve the cached Arc<CargoToml> by comparing entry.mtime == current_mtime (both SystemTime from metadata().modified()). On filesystems with second-resolution mtime (HFS+, FAT/exFAT, NFS with old actimeo), two writes inside the same second produce identical mtimes — the cache happily serves the pre-edit manifest for the rest of the second.

**Why it matters**: Long-running daemon shapes (CI worker, language-server-style host) are exactly the LRU-cap (TASK-0843) deployment target, and they are most likely to do back-to-back manifest edits within a single mtime tick. Pairing mtime with file size or a content hash captured on insert removes the false-fresh window.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test simulates two distinct manifests stamped with identical mtimes (manually via filetime::set_file_mtime) and asserts the second load_workspace_manifest reparses rather than returning the first cached Arc.
- [ ] #2 The freshness key includes both mtime and either file length or a cheap content hash; existing tests for refresh / cross-thread sharing continue to pass.
<!-- AC:END -->
