---
id: TASK-0029
title: 'CD-1: DuckDB query functions share 65% scaffolding boilerplate'
status: Done
assignee: []
created_date: '2026-04-14 19:17'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-2
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions/duckdb/src/sql.rs — 7 query_* functions (query_project_file_count, query_crate_file_count, query_project_loc, query_dependency_count, query_project_languages, query_crate_loc, query_crate_coverage) repeat the same lock→table_exists→VALUES CTE→prepare→query_map→loop/insert scaffolding. Boilerplate ranges from 14 lines (simple queries) to 39 lines (query_crate_coverage). Unique query logic is only 1-18 lines per function. The .context('reading X row')? error mapping also repeats identically at 6 call sites. Violates DUP-2 (3+ structurally similar functions). Affected crate: ops-duckdb.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a shared query helper (e.g., query_with_cte_map) that encapsulates lock→table_exists→CTE→prepare→map. Each query function should reduce to the unique SQL + mapping closure.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
CD audit re-confirmation: sql.rs per-crate query functions (query_crate_file_count:338-389, query_crate_loc:483-529, query_crate_coverage:579-662) share identical skeleton: validate paths → build VALUES CTE → LEFT JOIN on starts_with → GROUP BY → collect into HashMap. Project-level scalar queries (query_project_file_count:312, query_project_loc:392, query_dependency_count:415, query_project_coverage:540) also share: lock → table_exists guard → query_row with single aggregate. Total: 7 functions with 65%+ scaffolding overlap.
<!-- SECTION:NOTES:END -->
