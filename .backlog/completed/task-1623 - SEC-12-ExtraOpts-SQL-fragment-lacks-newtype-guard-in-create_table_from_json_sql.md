---
id: TASK-1623
title: >-
  SEC-12: ExtraOpts SQL fragment lacks newtype guard in
  create_table_from_json_sql
status: Done
assignee:
  - TASK-1640
created_date: '2026-05-22 07:07'
updated_date: '2026-05-22 13:43'
labels:
  - code-review-rust
  - security
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest/sql.rs:11-33`

**What**: `extra_opts` (`Option<&str>`) is validated by `validate_extra_opts` then interpolated verbatim into `read_json_auto(..., {opts})`. The function is `pub`; current callers pass static literals, but any future dynamic caller (config-sourced opts) re-opens the SQL-injection surface the allowlist only narrows.

**Why it matters**: Defense-in-depth in this crate rests on "never interpolate untrusted input." A `pub fn` taking `Option<&str>` and pasting into SQL is a latent foot-gun. Encoding the validation contract in the type system mirrors `TableName::from_static` and prevents accidental bypass.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce an ExtraOpts newtype with a validating constructor (no public field access)
- [ ] #2 Replace the Option<&str> parameter in create_table_from_json_sql with the newtype
- [ ] #3 Add a doc comment on ExtraOpts describing the SQL interpolation contract
- [ ] #4 Test demonstrates a dynamic (non-static) validated construction path
<!-- AC:END -->
