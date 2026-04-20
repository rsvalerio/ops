---
id: TASK-0095
title: >-
  DUP-2: query_project_coverage and query_crate_coverage share coverage SUM/CASE
  SQL
status: Done
assignee: []
created_date: '2026-04-17 11:33'
updated_date: '2026-04-17 14:56'
labels:
  - rust-codereview
  - dup
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query.rs:259`

**What**: The COALESCE(SUM(...)) CASE WHEN ... block appears verbatim in both query_project_coverage (259-275) and query_crate_coverage (305-318).

**Why it matters**: Two places to update when coverage math changes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a coverage_select_fragment() -> &static str helper
- [ ] #2 Use it in both functions
<!-- AC:END -->
