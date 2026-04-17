---
id: TASK-0026
title: 'CQ-2: query_crate_coverage exceeds FN-1 and FN-2 thresholds'
status: Done
assignee: []
created_date: '2026-04-14 19:13'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-quality
  - FN-1+FN-2
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
extensions/duckdb/src/sql.rs:579-662 (83 lines) — query_crate_coverage builds a VALUES CTE, executes a multi-join SQL query, and maps rows to CrateCoverage structs with nested closures. 5+ nesting levels in the row-mapping closure. The same scaffolding pattern (lock DB → check table → build VALUES CTE → query → map rows) is repeated across 7 query functions in this file. Violates FN-1 (≤50 lines) and FN-2 (≤4 nesting). Affected crate: ops-duckdb.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract CTE-building logic into a helper. Flatten the row-mapping closure. Consider a shared query scaffolding helper for the repeated lock→check→build→query pattern.
<!-- AC:END -->
