---
id: TASK-0530
title: >-
  PERF-1: query_identity_metrics calls ops_duckdb::get_db three times per
  provide()
status: To Do
assignee:
  - TASK-0533
created_date: '2026-04-28 06:53'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/identity/metrics.rs:15`

**What**: query_identity_metrics calls query_dependency_count, query_coverage_and_languages, and query_loc_from_db, each independently calling ops_duckdb::get_db(ctx). Each call re-locates / re-locks the database handle.

**Why it matters**: Same anti-pattern as the existing about/units enrich_from_db lock-thrash task — three separate get_db round-trips per provide() when one shared handle would suffice. Runs on every `ops about`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 query_identity_metrics resolves get_db once and threads the borrowed handle to the sub-queries
- [ ] #2 Sub-functions accept &DuckDb instead of &Context
<!-- AC:END -->
