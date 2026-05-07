---
id: TASK-1116
title: >-
  ERR-1: query_project_languages errors instead of returning empty when total
  LOC is zero but files exist
status: Done
assignee: []
created_date: '2026-05-07 21:52'
updated_date: '2026-05-07 23:25'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/loc.rs:73`

**What**: The query divides by `NULLIF(totals.total_loc, 0)` so `loc_pct` becomes SQL NULL when every row in `tokei_files` has `code = 0` (a project of all-comment / all-blank / header-only files in a language). The row mapper then calls `row.get::<_, f64>(3)?` on the NULL, which errors with a duckdb type error; the error propagates out of `query_project_languages` and surfaces as a `tracing::warn!` from `query_language_stats`, giving operators a misleading "language stats failed" log instead of the documented "empty result" signal.

**Why it matters**: The contract documented above the function (READ-5 / TASK-0362) is "languages whose loc_pct rounds below 0.1% are omitted, *including* the case where every language is sub-threshold. The empty return is now the only signal." A NULL loc_pct violates that contract — instead of empty Ok(vec![]), the caller gets Err. Fix: COALESCE the NULL to 0.0 inside SQL (`COALESCE(ROUND(...), 0)`) so the >= 0.1 filter naturally drops the row and the empty-result path stays the only signal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 query_project_languages returns Ok(empty Vec) when tokei_files exists but every code value is 0, instead of erroring on a NULL loc_pct
<!-- AC:END -->
