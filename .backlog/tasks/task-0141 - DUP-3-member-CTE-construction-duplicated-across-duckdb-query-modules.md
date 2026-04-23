---
id: TASK-0141
title: 'DUP-3: member CTE construction duplicated across duckdb query modules'
status: To Do
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - dup
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/helpers.rs:207-213` and `extensions/duckdb/src/sql/query/coverage.rs:63-72`

**What**: Two functions build nearly identical `WITH members(path) AS (VALUES {placeholders})` CTEs with separate placeholder-generation code.

**Why it matters**: If the member CTE shape changes (column name, quoting, escaping), both sites must be updated in lockstep.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a single build_members_cte(paths) -> (sql, params) helper used by both query_per_crate_i64 and query_crate_coverage
- [ ] #2 Both call sites use the helper and pass only the SELECT expression / aggregate
<!-- AC:END -->
