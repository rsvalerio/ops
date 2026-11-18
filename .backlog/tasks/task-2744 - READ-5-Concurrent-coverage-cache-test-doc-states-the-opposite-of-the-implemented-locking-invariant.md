---
id: TASK-2744
title: 'READ-5: Concurrent coverage-cache test doc states the opposite of the implemented locking invariant'
status: Done
assignee: []
created_date: '2026-11-18 18:38'
updated_date: '2026-11-18 22:29'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2849'
modified_files:
  - extensions-rust/about/src/coverage_provider.rs
priority: low
ordinal: 1000
dedup_key: 'READ-5:extensions-rust/about/src/coverage_provider.rs:project_coverage_warn_fires_once_under_concurrent_first_callers'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:300-306` (doc on `cache_tests::project_coverage_warn_fires_once_under_concurrent_first_callers`)

**What**: The test doc says "The outer mutex must be held across the query: dropping it would let both threads observe a miss, both dispatch". The implementation (`cached_query_project_coverage`, and its own doc) does the opposite on purpose: the outer mutex is released after `slot_for`, the query runs outside it, and the per-key `OnceLock::get_or_init` is what guarantees one query / one warn.

**Why it matters**: The test pins the right behaviour but documents the wrong invariant; a reader trusting it could "fix" the code by holding the global mutex across a SQLite query, serialising every provider behind it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The test doc describes the OnceLock-per-slot guarantee with the query running outside the outer mutex, matching cached_query_project_coverage

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.11 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
