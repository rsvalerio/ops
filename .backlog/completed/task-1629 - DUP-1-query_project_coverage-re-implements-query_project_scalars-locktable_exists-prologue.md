---
id: TASK-1629
title: >-
  DUP-1: query_project_coverage re-implements query_project_scalar's
  lock+table_exists prologue
status: Done
assignee:
  - TASK-1640
created_date: '2026-05-22 07:12'
updated_date: '2026-05-22 13:43'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/coverage.rs:14-30` (vs `extensions/duckdb/src/sql/query/helpers.rs:136-154`)

**What**: `query_project_coverage` re-implements the lock → `table_exists` preflight → `query_row` → `with_context` scaffolding that `helpers::query_project_scalar` already abstracts for the LOC/deps/per-crate paths. The only structural difference is row arity: coverage reads three columns (covered/missed/total) and constructs a `CrateCoverage`, while the helper returns a single i64.

**Why it matters**: Two copies of the prologue means future hardening (e.g. tightening the table_exists semantics, adding an early-return guard for missing tables, or improving the error message) has to be applied in two places. A `query_project_row<F, T>` helper that takes a row mapper would let coverage share the same prologue used by `loc.rs` and `deps.rs`.

**Notes**: This pairs with the broader query-helpers pattern that already centralises per-crate queries via `PerCrateI64Query` / `query_per_crate_i64`. Extending the same abstraction to project-scalar/row queries finishes the refactor.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A generalised query_project_row<F, T> helper exists in helpers.rs and centralises lock+table_exists+query_row+with_context
- [ ] #2 query_project_coverage uses the new helper (or query_project_scalar delegates to it)
- [ ] #3 All existing tests for project coverage, LOC, and deps pass unchanged
<!-- AC:END -->
