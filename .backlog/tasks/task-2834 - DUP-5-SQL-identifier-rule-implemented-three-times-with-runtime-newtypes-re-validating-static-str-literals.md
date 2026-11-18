---
id: TASK-2834
title: 'DUP-5: SQL identifier rule implemented three times, with runtime newtypes re-validating &''static str literals'
status: To Do
assignee: []
created_date: '2026-11-18 18:50'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - DUP
dependencies: []
parent_task_id: 'TASK-2853'
modified_files:
  - extensions/sqlite/src/sql/validation.rs
  - extensions/sqlite/src/sql/ingest/sql.rs
  - extensions/sqlite/src/sql/query/helpers.rs
  - extensions/sqlite/src/sql/query/coverage.rs
  - extensions/sqlite/src/sql/query/loc.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-5:extensions/sqlite/src/sql/validation.rs:validate_identifier'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/validation.rs:64-78` (`validate_identifier`), `:129-146` (`is_valid_identifier_const`), `extensions/sqlite/src/sql/ingest/sql.rs:225-236` (`is_nonempty_identifier_bytes`), `extensions/sqlite/src/sql/query/helpers.rs:12-47` (`sql_ident_newtype!`: `QueryTableName`, `ColumnAlias`, `ColumnName`), `extensions/sqlite/src/sql/query/coverage.rs:87-88`, `extensions/sqlite/src/sql/query/loc.rs:45-49,298-302`

**What**: The `[A-Za-z_][A-Za-z0-9_]*` rule is coded three times: a runtime `chars()` loop, a const byte-slice walk, and a third const walk for the identifier tail. All three must stay in lockstep by hand. On top of that, the query side defines three macro-generated newtypes that wrap a `&'static str` but validate at *runtime*. As a result every call site that passes a string literal either propagates an impossible error with `?` (`QueryTableName::new("tokei_files")?`, `ColumnAlias::new("f")?`) or unwraps it under `#[allow(clippy::expect_used)]` with no `reason` (`coverage.rs:87`). The function then has to document a `# Panics` section for a literal that can never fail. `TableName::from_static` already provides the compile-time-validated form for exactly this input type.

**Why it matters**: Drift risk between three validators that guard SQL interpolation, plus dead error paths and panic docs that make the query API look fallible where it is not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 One function owns the identifier rule; validate_identifier and the JsonColumn path check delegate to it
- [ ] #2 Query identifiers built from literals are const-validated (TableName-style), so query_crate_loc / query_crate_file_count / query_crate_coverage have no identifier error path and coverage.rs drops the expect and its allow
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.30 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
