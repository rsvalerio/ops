---
id: TASK-1571
title: >-
  TEST-25: distinct_db_instances_do_not_alias_cache_keys asserts DuckDb::id
  invariant, not the project_coverage_cache it claims to guard
status: To Do
assignee:
  - TASK-1578
created_date: '2026-05-19 16:35'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:329-339`

**What**: The test is labelled "ARCH-9 / TASK-1155: two distinct DuckDb instances must receive distinct cache keys even when one is dropped and the next is allocated at the same memory address (the ABA hazard the prior pointer-address scheme had)" but the body only checks `assert_ne!(a.id(), b.id())`. It never:
- inserts into `project_coverage_cache`,
- drops `a` and observes whether the cache entry survives,
- forces address reuse for `b`,
- or asserts that `cached_query_project_coverage(&b)` does NOT surface `a`'s cached value.

The assertion is on a `DuckDb::id` property and would still pass even if `cached_query_project_coverage` keyed by raw pointer address — the very regression the test claims to prevent.

**Why it matters**: A future refactor that re-keys `project_coverage_cache` on `&db as *const _ as usize` would re-introduce the ABA hazard TASK-1155 fixed, and this test would happily pass because it never touches the cache. The test name advertises a contract it does not pin.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test asserts via cached_query_project_coverage call sites that distinct DuckDb instances do not share cached values (not just distinct id())
- [ ] #2 Test demonstrates the ABA-resistance contract — e.g. by priming the cache against the first instance, dropping it, and verifying a new instance does not observe the cached payload
- [ ] #3 Or the test is renamed / split so the DuckDb::id invariant test lives alongside the DuckDb crate while the cache aliasing test lives where the cache lives
<!-- AC:END -->
