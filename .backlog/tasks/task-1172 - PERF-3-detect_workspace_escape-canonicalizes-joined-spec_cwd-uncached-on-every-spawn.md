---
id: TASK-1172
title: >-
  PERF-3: detect_workspace_escape canonicalizes joined spec_cwd uncached on
  every spawn
status: Done
assignee:
  - TASK-1263
created_date: '2026-05-08 08:06'
updated_date: '2026-05-09 11:08'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:290`

**What**: `detect_workspace_escape` calls `std::fs::canonicalize(joined)` for every spawn (line 290), bypassing the `WorkspaceCanonicalCache`. Only the workspace side is cached. With many parallel spawns sharing the same spec cwd (e.g. composite that fans the same `cwd = "sub"` 32 times), every spawn pays a fresh canonicalize syscall.

**Why it matters**: Counter to the PERF-3/TASK-0765 intent of avoiding canonicalize on the spawn hot path. On NFS or symlink-heavy paths the per-spawn cost dominates `build_command_async`'s blocking-pool dispatch.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The joined-path canonicalize result is reused across spawns that share the same joined path within a runner (or a documented short TTL), via the same cache type.
- [x] #2 Symlink-swap regression continues to be detected after invalidation (mirror of TASK-1063 AC #3).
<!-- AC:END -->
