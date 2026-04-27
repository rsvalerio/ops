---
id: TASK-0406
title: >-
  SEC-12: coverage_col_select interpolates caller-supplied prefix into SQL with
  no validation
status: To Do
assignee:
  - TASK-0419
created_date: '2026-04-26 09:52'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/helpers.rs:62-70`

**What**: `coverage_col_select(prefix: &str)` formats `prefix` directly into the COALESCE/SUM/CASE expressions used by `query_project_coverage` and `query_crate_coverage`. Today both call sites pass either `""` or `"c."`, so there is no live exploit path, but the function accepts an unvalidated `&str` and the surrounding query module has adopted a typed-newtype pattern (`TableName`, `ColumnAlias`, `ColumnName`) precisely to prevent this class of regression. The `coverage_col_select` helper is the one remaining hole: a future refactor that lets a caller forward a column name through this path would silently re-open SQL injection.

**Why it matters**: Defense-in-depth gap that the rest of this module has already closed for identifiers. Cheap to fix (accept a `ColumnAlias` or validate the prefix against `^[a-zA-Z_][a-zA-Z0-9_]*\.?$`), and aligns with the SEC-12 / API-1 stance the rest of the module enforces.

<!-- scan confidence: high; both call sites read, design intent inferred from sibling newtype helpers -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 coverage_col_select rejects or refuses any non-validated prefix
- [ ] #2 tests cover both legitimate prefixes ("" and "c.") and a rejected non-conforming prefix
<!-- AC:END -->
