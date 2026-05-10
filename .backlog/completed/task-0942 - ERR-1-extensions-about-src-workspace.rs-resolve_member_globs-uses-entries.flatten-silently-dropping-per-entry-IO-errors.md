---
id: TASK-0942
title: >-
  ERR-1: extensions/about/src/workspace.rs::resolve_member_globs uses
  entries.flatten(), silently dropping per-entry IO errors
status: Done
assignee: []
created_date: '2026-05-02 16:02'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:40`

**What**: Inside the `Ok(entries)` arm of the workspace-glob walk, `for entry in entries.flatten()` discards per-entry IO errors (EACCES on a sibling member, EIO, etc.) without logging. The outer `Err(e)` arm at lines 61-67 already logs read_dir-level failures, but the per-entry arm has no equivalent.

**Why it matters**: A permissions-denied member directory becomes "no project units found" with zero diagnostic. Same pattern as TASK-0935 (Stack::has_manifest_in_dir) but in a different file and code path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace entries.flatten() with explicit match { Ok | Err(e) => tracing::warn!(...) } arm, matching TASK-0517 policy in the same file
- [x] #2 Regression test confirms a per-entry permission error logs at warn level rather than being silently dropped
<!-- AC:END -->
