---
id: TASK-1063
title: >-
  CONC-7: Workspace canonicalize cache in CommandRunner is unbounded and
  process-global
status: Done
assignee: []
created_date: '2026-05-07 21:17'
updated_date: '2026-05-08 00:00'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/build.rs:33-47`

**What**: A `OnceLock<RwLock<HashMap<PathBuf, Option<PathBuf>>>>` caches `canonicalize(workspace)` results forever, keyed by raw input `PathBuf`. The comment claims one key in production, but tests inject many tempdirs and any future library embedding (or in-process integration tests) accumulates entries indefinitely.

**Why it matters**: Symlink swaps after caching produce stale containment decisions on subsequent calls — a SEC-25-shaped escape window the cache widens. Process-global state also makes runner lifetime invisible to the test fixture.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Bound the cache (LRU or runner-scoped) so repeated calls cannot grow without limit
- [x] #2 Fold the cache into CommandRunner so its lifetime ends with the runner
- [x] #3 Add a regression test that swapping a symlink under a cached workspace path is re-canonicalized
<!-- AC:END -->
