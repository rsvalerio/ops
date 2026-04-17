---
id: TASK-0083
title: >-
  FN-1: query_crate_coverage is ~57 lines and mixes validation, SQL building,
  row mapping
status: Done
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 14:56'
labels:
  - rust-codereview
  - fn
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query.rs:283`

**What**: query_crate_coverage spans lines 283-340 handling path validation, setup match, SQL format!, prepare/query_map, and result collection.

**Why it matters**: Exceeds FN-1 and mixes abstraction levels; near-duplicate of query_per_crate_i64 differing only in row shape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Generalize query_per_crate_i64 to take a row-mapper closure returning T, and reuse it here
- [ ] #2 Extract the SQL construction into a helper
<!-- AC:END -->
