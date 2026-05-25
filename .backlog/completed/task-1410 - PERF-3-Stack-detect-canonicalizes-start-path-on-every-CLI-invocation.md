---
id: TASK-1410
title: 'PERF-3: Stack::detect canonicalizes start path on every CLI invocation'
status: Done
assignee:
  - TASK-1451
created_date: '2026-05-13 18:17'
updated_date: '2026-05-13 20:01'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack/detect.rs:95`

**What**: `detect()` calls `std::fs::canonicalize(start)` unconditionally on every CLI dispatch. `canonicalize` performs one stat per path component and dereferences every symlink. On a deep cwd or NFS/FUSE mount this is many syscalls per `ops <cmd>` invocation.

**Why it matters**: Stack detection runs once per CLI invocation on the critical startup path; syscall fan-out matters under slow filesystems and tools that shell out to `ops` repeatedly. The canonical workspace root is already cached elsewhere (TASK-1063/1229) — extend that cache (or a per-process OnceLock keyed by `start`) so detection skips canonicalize after the first call.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 memoize canonicalized start path so detect's canonicalize fires at most once per (process, start) pair
- [x] #2 preserve fallback-to-lexical-walk behaviour when canonicalize errors, matching the existing tracing::debug breadcrumb
- [x] #3 add a regression test or counter pinning that repeat detect() calls do not re-issue canonicalize
<!-- AC:END -->
