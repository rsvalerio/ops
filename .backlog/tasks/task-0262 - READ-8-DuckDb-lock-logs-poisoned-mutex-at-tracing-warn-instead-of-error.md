---
id: TASK-0262
title: 'READ-8: DuckDb::lock logs poisoned mutex at tracing::warn instead of error'
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 09:17'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/connection.rs:79`

**What**: warn is likely wrong severity for a poisoned mutex; operators may miss a critical correctness failure.

**Why it matters**: Data loss signal warrants error level.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Use tracing::error! and include db_path
- [x] #2 Cross-reference task-0196 for caller context
<!-- AC:END -->
