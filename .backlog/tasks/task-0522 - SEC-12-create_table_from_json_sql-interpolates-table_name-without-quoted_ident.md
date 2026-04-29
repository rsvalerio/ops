---
id: TASK-0522
title: >-
  SEC-12: create_table_from_json_sql interpolates table_name without
  quoted_ident
status: Done
assignee:
  - TASK-0534
created_date: '2026-04-28 06:52'
updated_date: '2026-04-28 19:01'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:25`

**What**: `CREATE OR REPLACE TABLE {table_name} AS ...` interpolates a validated but unquoted identifier; the same module's table_has_data and drop_table_if_exists use quoted_ident for defense-in-depth.

**Why it matters**: validate_identifier already enforces [A-Za-z_][A-Za-z0-9_]*, so this is currently safe — but the inconsistent pattern means a future widening of validate_identifier (e.g. allowing `.` for schema-qualified names) silently breaks the safety contract here while the other sites stay safe.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use quoted_ident(table_name)? and interpolate the quoted form
- [ ] #2 Tests assert SQL contains a double-quoted identifier
<!-- AC:END -->
