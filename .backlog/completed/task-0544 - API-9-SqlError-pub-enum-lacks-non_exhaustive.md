---
id: TASK-0544
title: 'API-9: SqlError pub enum lacks #[non_exhaustive]'
status: Done
assignee:
  - TASK-0636
created_date: '2026-04-29 04:58'
updated_date: '2026-04-29 06:12'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/validation.rs:33`

**What**: `pub enum SqlError` is re-exported through `ops_duckdb::sql::SqlError` (used by tokei::views) but lacks `#[non_exhaustive]`. Adding a new error variant would be a breaking change for downstream extensions matching exhaustively.

**Why it matters**: Extension-facing error types must be evolvable without major-version bumps.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 SqlError is annotated #[non_exhaustive]
- [ ] #2 An exhaustive match over SqlError from outside the crate now requires a wildcard arm
<!-- AC:END -->
