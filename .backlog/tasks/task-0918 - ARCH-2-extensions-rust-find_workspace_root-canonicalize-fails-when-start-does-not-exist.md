---
id: TASK-0918
title: >-
  ARCH-2: extensions-rust find_workspace_root canonicalize fails when start does
  not exist
status: Done
assignee: []
created_date: '2026-05-02 10:12'
updated_date: '2026-05-02 14:58'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/lib.rs:287`

**What**: fs::canonicalize(start) at the top of find_workspace_root errors if `start` (working_directory) is missing or a dangling symlink. CargoTomlProvider::provide therefore propagates a `failed to canonicalize` error in cases where the previous behavior silently walked the lexical parents. This breaks About on a transient cwd unlink (CI volume eviction, watcher rename).

**Why it matters**: Workspace discovery is foundational; an unreadable cwd should produce a clean NotFound that the chain handles, not a confusing canonicalize message that hides which step failed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 canonicalize failure routes to a typed NotFound-shaped error (or falls back to lexical walk with a debug log) so downstream code distinguishes no-manifest from cwd-unreachable
- [x] #2 Test covers a deleted-cwd path
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
find_workspace_root distinguishes ErrorKind::NotFound from canonicalize (transient cwd unlink, dangling symlink, deleted-cwd) — reported as FindWorkspaceRootError::NotFound so downstream is_manifest_missing routes through the same branch as a regular missing-Cargo.toml. Other IO errors (PermissionDenied, IsADirectory) keep the typed CanonicalizeFailed variant. Updated the misnamed test from CanonicalizeFailed-on-missing to NotFound-on-missing, and added find_root_canonicalize_perm_denied_keeps_canonicalize_failed_variant pinning the residual investigable path.
<!-- SECTION:NOTES:END -->
