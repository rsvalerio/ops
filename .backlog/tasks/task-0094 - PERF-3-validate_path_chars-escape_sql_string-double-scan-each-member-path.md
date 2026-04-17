---
id: TASK-0094
title: 'PERF-3: validate_path_chars + escape_sql_string double-scan each member path'
status: To Do
assignee: []
created_date: '2026-04-17 11:33'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query.rs:65`

**What**: For every per-crate query, prepare_per_crate iterates member_paths and char-scans each; then escape_sql_string does another char-scan.

**Why it matters**: Minor CPU; more importantly the double-scan duplicates validation logic already proven sufficient.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Combine validate_path_chars + escape_sql_string into a single pass returning the escaped string
- [ ] #2 Or move to parameterized binding (see SEC-12 finding)
<!-- AC:END -->
