---
id: TASK-1465
title: >-
  PERF-3: cached_ops_root_arc canonicalize syscall fires on every from_env
  before cache lookup
status: To Do
assignee:
  - TASK-1481
created_date: '2026-05-15 18:50'
updated_date: '2026-05-17 07:06'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:138-156`

**What**: The hit path documented in TASK-1423 is described as "borrow-by-`&Path` fast path", but every call to `cached_ops_root_arc` first runs `std::fs::canonicalize(ops_root)` *before* probing the cache. The cache hit then merely avoids the allocation, not the syscall. Hooks (`run-before-commit`/`run-before-push`) and dry-run paths invoke `from_env` repeatedly, each time re-traversing the workspace root via `realpath(2)`.

**Why it matters**: The documented motivation for the cache is to amortise repeated work; the unconditional canonicalize syscall wipes out most of that gain on the dominant single-root workload. Probe the cache by raw `&Path` first; only canonicalize on miss to install the alias entry.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cached_ops_root_arc probes guard.map by the raw &Path first; canonicalize runs only on miss to install the alias entry
- [ ] #2 Existing ops_root_cache_hit_path_reuses_arc test (or a sibling) is extended with a counter (or syscall fault) demonstrating zero canonicalize calls after warm-up for the same path
<!-- AC:END -->
