---
id: TASK-0198
title: 'API-1: PerCrateI64Query uses raw &str for table/alias/column — swap-prone'
status: Done
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 08:50'
labels:
  - rust-code-review
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/helpers.rs:179-218`

**What**: PerCrateI64Query has 7 fields including similarly-typed table, join_alias, join_column, label. All &str; nothing prevents swapping join_alias and join_column at construction. No type-level distinction between a table name, an alias, and a column name.

**Why it matters**: API-1/API-2 — primitive obsession on SQL identifiers. Newtypes TableName, ColumnAlias, ColumnName with validated constructors would catch swaps at compile time.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 SQL identifier fields use newtype wrappers with validated constructors
- [x] #2 Constructors enforce validate_identifier once, removing runtime checks at use sites
<!-- AC:END -->
