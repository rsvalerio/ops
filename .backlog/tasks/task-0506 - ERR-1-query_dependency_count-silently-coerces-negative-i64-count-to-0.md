---
id: TASK-0506
title: 'ERR-1: query_dependency_count silently coerces negative i64 count to 0'
status: To Do
assignee:
  - TASK-0534
created_date: '2026-04-28 06:51'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/deps.rs:16`

**What**: `usize::try_from(count).unwrap_or(0)` discards the TryFromIntError so a negative i64 from COUNT (anomaly / cast bug) is reported as 0 deps with no log or error.

**Why it matters**: Other count-conversion sites (ingestor.rs InvalidRecordCount) surface this as a typed error; here it is silently masked, making bad data look like an empty project.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace unwrap_or(0) with an error path or tracing::warn! and 0
- [ ] #2 Regression test: planted negative scalar surfaces as warn/err
<!-- AC:END -->
