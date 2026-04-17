---
id: TASK-0081
title: >-
  DUP-3: DuckDB downcast pattern still duplicated in try_provide_from_db and
  get_db
status: To Do
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - dup
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/lib.rs:38`

**What**: ctx.db.as_ref().and_then(|h| h.as_any().downcast_ref::<DuckDb>()) is repeated verbatim in both try_provide_from_db and get_db. Prior task-0032 reduced from 5 to 2; residual duplication remains.

**Why it matters**: Shotgun edits to the downcast contract must be made in multiple spots.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Have try_provide_from_db call get_db(ctx) internally instead of re-implementing the downcast
- [ ] #2 Confirm borrow semantics still allow the db_fn closure to take &DuckDb
<!-- AC:END -->
