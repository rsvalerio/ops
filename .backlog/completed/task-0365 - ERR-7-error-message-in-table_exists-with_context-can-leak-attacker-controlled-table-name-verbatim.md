---
id: TASK-0365
title: >-
  ERR-7: error message in table_exists with_context can leak attacker-controlled
  table name verbatim
status: Done
assignee:
  - TASK-0419
created_date: '2026-04-26 09:36'
updated_date: '2026-04-27 10:53'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:52`

**What**: with_context(|| format!("checking if {} exists", table_name)) echoes the unvalidated table_name into the error chain. table_exists is called before identifier validation in provide_via_ingestor.

**Why it matters**: Minor information disclosure / log-injection vector if table_name ever flows from user input. Today all callers pass static strings; defense-in-depth.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either validate table_name at the top of table_exists or sanitize/quote it in the error context
- [x] #2 Test passes a control-character-laden table name and asserts the error message is sanitized
<!-- AC:END -->
