---
id: TASK-0175
title: 'SEC-12: table name interpolation repeated in three sites without shared helper'
status: Done
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 08:42'
labels:
  - rust-code-review
  - SEC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:46-64,184-192`, `extensions/duckdb/src/ingestor.rs:82-91`

**What**: table_has_data, drop_table_if_exists, and load_with_sidecar each validate an identifier then format it into a double-quoted table reference. The shared invariant is implicit across three call sites.

**Why it matters**: SEC-12 defense-in-depth + DUP-3. Extract a single helper fn quoted_ident(name) that returns the escaped quoted identifier once, so callers cannot forget validation or bypass it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All table-name interpolation goes through a single validated quoting helper
- [x] #2 Call sites use the helper rather than ad-hoc validate_identifier plus format
<!-- AC:END -->
