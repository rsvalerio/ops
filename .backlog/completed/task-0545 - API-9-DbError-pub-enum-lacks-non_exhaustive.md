---
id: TASK-0545
title: 'API-9: DbError pub enum lacks #[non_exhaustive]'
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
**File**: `extensions/duckdb/src/error.rs:8`

**What**: `pub enum DbError` is re-exported as `ops_duckdb::DbError` and used by every ingestor extension, but is missing `#[non_exhaustive]`. New variants (e.g. richer timeout context) cannot be added without breaking out-of-crate exhaustive matches.

**Why it matters**: This is the database error type seen by every dependent extension crate; same hazard as existing API-9 filings.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DbError carries #[non_exhaustive]
- [ ] #2 Existing internal matches still compile after the annotation
<!-- AC:END -->
