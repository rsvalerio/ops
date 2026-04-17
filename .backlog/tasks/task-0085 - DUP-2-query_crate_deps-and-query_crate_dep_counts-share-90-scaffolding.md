---
id: TASK-0085
title: 'DUP-2: query_crate_deps and query_crate_dep_counts share 90% scaffolding'
status: Done
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 14:56'
labels:
  - rust-codereview
  - dup
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query.rs:346`

**What**: query_crate_deps (346-385) and query_crate_dep_counts (391-423) both lock, check table_exists, prepare a similar statement, iterate, and collect — differing only in SELECT columns and row shape.

**Why it matters**: Duplication of the already-generalized query_rows_to_map pattern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce query_rows_to_map<K, V, F> in sql module and reuse it in both functions
- [ ] #2 Consider reusing query_rows_to_json where the caller already returns JSON
<!-- AC:END -->
