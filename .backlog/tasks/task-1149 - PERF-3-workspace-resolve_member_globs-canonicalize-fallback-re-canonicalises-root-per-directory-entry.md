---
id: TASK-1149
title: >-
  PERF-3: workspace::resolve_member_globs canonicalize fallback re-canonicalises
  root per directory entry
status: Done
assignee:
  - TASK-1263
created_date: '2026-05-08 07:43'
updated_date: '2026-05-09 11:05'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:110-141`

**What**: When path.strip_prefix(root) misses (macOS /var vs /private/var, or any symlinked workspace root), the recovery path runs std::fs::canonicalize(root) AND std::fs::canonicalize(&path) per entry inside the read_dir loop. canonicalize(root) returns the same value every iteration and is hoist-able.

**Why it matters**: Hot path on monorepos with symlinked roots. canonicalize is a non-trivial syscall sequence; doing it N times per glob expansion turns one O(1) recovery into O(N) syscalls.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Lazily canonicalize root once on first miss and cache the OnceCell across the loop
- [x] #2 Add a test mounting a 200-entry symlinked-root tree and asserting canonicalize is called at most twice
<!-- AC:END -->
