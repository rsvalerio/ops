---
id: TASK-0431
title: 'ERR-1: about/units enrich_from_db re-locks DuckDB across 4 independent queries'
status: Done
assignee:
  - TASK-0534
created_date: '2026-04-28 04:42'
updated_date: '2026-04-28 18:46'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/units.rs:76-97` (and parallel pattern in `extensions/about/src/lib.rs:130-167` for the project-level enrich_from_db)

**What**: Each of query_crate_loc, query_crate_file_count, query_project_loc, query_project_file_count independently acquires db.lock(). A concurrent refresh / ingestion that runs between two of these queries can leave per-crate sums inconsistent with the project totals shown in the same render.

**Why it matters**: The user sees a rendered card that asserts "X loc total / Y per crate" where X and Y were sampled from different points in time. Not a safety issue, but a confusing output that is hard to reproduce.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either (a) acquire one lock and run all four queries under it, or (b) document that the four queries are independent samples and add a regression test asserting that the inconsistency is acceptable
- [ ] #2 Tests still pass; no new lock-held-across-.await regressions
<!-- AC:END -->
