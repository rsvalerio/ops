---
id: TASK-0259
title: >-
  ERR-10: SqlError::PathTraversalNotAllowed carries a String instead of a typed
  path
status: To Do
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/validation.rs:11`

**What**: Error variant uses raw String for path; diagnostics lose PathBuf context.

**Why it matters**: Violates ERR-10 spirit and makes downstream matching harder.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use PathBuf in the variant
- [ ] #2 Update tests/consumers
<!-- AC:END -->
