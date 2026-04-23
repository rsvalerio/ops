---
id: TASK-0254
title: 'READ-5: table_exists misses views used as data sources'
status: To Do
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:30`

**What**: Only checks information_schema.tables; views like crate_dependencies may be missed so downstream query_* helpers short-circuit to "no data".

**Why it matters**: Views-backed data sources may report empty without executing the real query.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Query information_schema.tables + information_schema.views (UNION) or duckdb_tables()/duckdb_views()
- [ ] #2 Regression test with a view-only schema
<!-- AC:END -->
