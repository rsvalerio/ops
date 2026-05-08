---
id: TASK-1229
title: >-
  CONC-2: WorkspaceCanonicalCache::get_or_compute holds Mutex across
  canonicalize() syscall
status: Done
assignee:
  - TASK-1261
created_date: '2026-05-08 12:58'
updated_date: '2026-05-08 14:54'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:120-160`

**What**: `get_or_compute` holds the cache Mutex across the user-supplied `canonicalize` closure (which performs `std::fs::canonicalize`). Concurrent first-time lookups for distinct workspace paths therefore serialize on the same mutex during the syscall. Mirrors the TASK-1144 pattern in ArcTextCache but in a different cache invoked on every parallel spawn via canonical_workspace_cached.

**Why it matters**: Under MAX_PARALLEL=32 with a multi-workspace embedder or test fixture, every cache miss stalls all peer spawns inside the same lock. The thundering-herd dedup is intentional, but it serializes distinct paths too.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Drop the mutex around the closure using an in-flight sentinel
- [ ] #2 OR document the per-call serialised contract
- [ ] #3 Regression test: two distinct workspace paths canonicalize concurrently
<!-- AC:END -->
