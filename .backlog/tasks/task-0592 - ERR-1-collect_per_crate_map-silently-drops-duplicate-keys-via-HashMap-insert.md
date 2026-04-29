---
id: TASK-0592
title: 'ERR-1: collect_per_crate_map silently drops duplicate keys via HashMap::insert'
status: Done
assignee:
  - TASK-0638
created_date: '2026-04-29 05:18'
updated_date: '2026-04-29 10:36'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/helpers.rs:251`

**What**: collect_per_crate_map builds HashMap<String, T> by `result.insert(path, val)`. If a query returns duplicate path rows (workspace glob bug, dropped GROUP BY), the second row silently overwrites the first. Sibling query_rows_fold has the same shape problem when fold_fn uses map.insert (e.g. query_crate_dep_counts deps.rs:108).

**Why it matters**: ERR-1. Duplicate-key collision is a query-shape invariant violation, not normal data, and should at least surface tracing::warn.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 collect_per_crate_map warns when insert returns Some(_)
- [ ] #2 Regression test pins behavior on deliberately ambiguous query
- [ ] #3 Same guard considered for query_rows_fold callers
<!-- AC:END -->
