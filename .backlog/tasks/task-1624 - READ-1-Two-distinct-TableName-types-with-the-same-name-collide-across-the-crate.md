---
id: TASK-1624
title: >-
  READ-1: Two distinct TableName types with the same name collide across the
  crate
status: Done
assignee:
  - TASK-1640
created_date: '2026-05-22 07:08'
updated_date: '2026-05-22 13:43'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/validation.rs:77-106` and `extensions/duckdb/src/sql/query/helpers.rs:38`

**What**: `validation::TableName` (const-validated, `&'static str`, public) and `query::helpers::TableName` (runtime-validated, crate-private, macro-generated) share the same identifier and both expose `as_str()`. Jump-to-def and reviewer search are fooled; a wrong `use` would compile silently while diverging the validation contract.

**Why it matters**: Identical names for distinct invariants is a classic READ-1/READ-6 trap. A future refactor merging or importing the wrong one will pass tests while changing semantics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Rename one type to be self-describing (e.g. QueryTableName or StaticTableName)
- [ ] #2 Update all in-crate call sites without use aliases that re-shadow the original name
- [ ] #3 Add a one-line doc comment on the surviving TableName describing its role
<!-- AC:END -->
