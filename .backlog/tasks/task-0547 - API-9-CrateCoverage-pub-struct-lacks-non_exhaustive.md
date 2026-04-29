---
id: TASK-0547
title: 'API-9: CrateCoverage pub struct lacks #[non_exhaustive]'
status: Triage
assignee: []
created_date: '2026-04-29 05:01'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/helpers.rs:44`

**What**: CrateCoverage is re-exported as ops_duckdb::sql::CrateCoverage and consumed by about-page renderers, but exposes all three fields as pub and has no #[non_exhaustive]. New coverage dimensions (functions, branches) cannot be added without a breaking change.

**Why it matters**: Extension data types must remain extensible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Struct annotated #[non_exhaustive]
- [ ] #2 A constructor (or builder) is added; zero() is preserved
<!-- AC:END -->
