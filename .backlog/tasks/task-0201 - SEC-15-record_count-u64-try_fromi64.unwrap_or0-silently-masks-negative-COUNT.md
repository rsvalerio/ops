---
id: TASK-0201
title: >-
  SEC-15: record_count u64::try_from(i64).unwrap_or(0) silently masks negative
  COUNT
status: To Do
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - SEC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:88-91`

**What**: After reading COUNT as i64, load_with_sidecar does u64::try_from(v).unwrap_or(0). A negative COUNT (anomaly, schema bug, signed overflow) would be silently reported as 0.

**Why it matters**: SEC-15 — make overflow/conversion failures explicit. Return DbError::query_failed with context instead of defaulting to 0.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Negative i64 COUNT returns a DbError instead of silently defaulting to 0
- [ ] #2 Test covers the negative-count error path using a mock
<!-- AC:END -->
