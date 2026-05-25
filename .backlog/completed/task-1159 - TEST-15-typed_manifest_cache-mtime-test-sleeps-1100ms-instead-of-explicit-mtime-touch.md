---
id: TASK-1159
title: >-
  TEST-15: typed_manifest_cache mtime test sleeps 1100ms instead of explicit
  mtime touch
status: Done
assignee:
  - TASK-1266
created_date: '2026-05-08 07:44'
updated_date: '2026-05-09 14:05'
labels:
  - code-review-rust
  - TEST
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:827`

**What**: `typed_manifest_cache_invalidates_on_mtime_change` sleeps 1100ms between two `fs::write` calls so HFS+/ext3 second-resolution mtimes advance. TEST-15 forbids sleep-based sync.

**Why it matters**: Adds 1.1s to every CI run; unreliable on filesystems with even-coarser mtime resolution (NFS, network mounts) and meaningless on filesystems with sub-second mtimes (ext4, APFS). Explicit `filetime::set_file_mtime` gives deterministic invalidation in microseconds.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Use filetime crate or std::fs::File::set_modified to bump mtime explicitly to a known-future timestamp
- [x] #2 Drop the sleep(Duration::from_millis(1100)) line
- [x] #3 Total test wall-time drops from >1.1s to <50ms
<!-- AC:END -->
