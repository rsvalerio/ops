---
id: TASK-1193
title: >-
  CONC-2: cached_query_project_coverage drops the lock around the query, racing
  duplicate warns
status: To Do
assignee:
  - TASK-1261
created_date: '2026-05-08 08:13'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - conc
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:43-63`

**What**: cached_query_project_coverage acquires the cache mutex, checks for a hit, drops the guard, then runs query_or_warn(...) outside any lock before re-acquiring the mutex to insert. Two providers entering this function in parallel both observe a miss, both dispatch query_project_coverage, and query_or_warn fires its warn N times — defeating the DUP-1 / TASK-1079 contract that "the warn fires exactly once per run".

**Why it matters**: The whole purpose of the per-process cache is dedup — both for cost (DuckDB scan) and for one-shot warn semantics. A drop-and-reacquire pattern around the work cannot achieve "exactly once". A OnceCell per DuckDb key (or Mutex<HashMap<usize, Arc<OnceCell<...>>>>) gives the documented "fires once" guarantee under concurrency.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Two threads invoking cached_query_project_coverage against the same &DuckDb cause query_project_coverage to execute exactly once and query_or_warn's warn to fire exactly once (verified with a tracing capture).
- [ ] #2 Existing project_coverage_warn_fires_once_across_both_call_sites test is extended to drive the two call sites from two threads, not sequentially, and still asserts warn_count == 1.
<!-- AC:END -->
