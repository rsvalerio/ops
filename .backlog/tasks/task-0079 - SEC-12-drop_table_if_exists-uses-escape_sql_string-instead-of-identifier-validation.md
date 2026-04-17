---
id: TASK-0079
title: >-
  SEC-12: drop_table_if_exists uses escape_sql_string instead of identifier
  validation
status: To Do
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - sec
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:185`

**What**: drop_table_if_exists uses validate_path_chars + escape_sql_string on the table name and interpolates it into a DROP TABLE statement, but permitted path chars are never valid SQL identifier characters.

**Why it matters**: Accepts structurally invalid identifiers and bypasses the stricter validate_identifier check; future misuse could lead to malformed DDL.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use validate_identifier(table_name) instead of validate_path_chars + escape_sql_string
- [ ] #2 Quote with proper DuckDB identifier quoting rules once validated
- [ ] #3 Add tests covering rejection of whitespace, dots, dashes, and injection attempts
<!-- AC:END -->
