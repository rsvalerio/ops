---
id: TASK-1418
title: 'CONC-1: ops_root_cache HashMap is unbounded and process-global'
status: To Do
assignee:
  - TASK-1455
created_date: '2026-05-13 18:17'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - conc
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/expand.rs:103`

**What**: `ops_root_cache` stores `Arc<str>` keyed by every distinct `ops_root: PathBuf` seen by `Variables::from_env`. The map is process-global with no eviction; a long-lived process (test binary running many parallel tests with distinct workspace roots, an embedder reusing `ops_core` across many projects) accumulates one entry per distinct root for the process lifetime.

**Why it matters**: The cache exists to avoid evicting under parallel tests, but unbounded growth is a memory leak for embedders that legitimately rotate `ops_root`. Add an LRU cap (e.g. 64 distinct roots) and document the eviction policy alongside the existing process-lifetime caveat.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cap ops_root_cache size with LRU eviction
- [ ] #2 the test serial-locked region continues to observe Arc::ptr_eq for the same root within the cap window
- [ ] #3 regression test inserts >cap distinct roots and asserts the cache stays bounded
<!-- AC:END -->
