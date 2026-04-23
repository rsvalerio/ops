---
id: TASK-0180
title: 'READ-1: query_crate_coverage dual-prefix join logic buried in SQL template'
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/coverage.rs:40-84`

**What**: query_crate_coverage inlines a SQL template joining on both starts_with(c.filename, m.path plus slash) and starts_with(c.filename, workspace_root plus slash plus m.path plus slash) to accommodate LLVM coverage outputs that may be absolute or relative. The dual-prefix matching rule is a domain invariant buried in a SQL format string. Not documented at the join, only in the function-level doc, and not unit-tested for both branches.

**Why it matters**: READ-1/READ-5. A future change to coverage filename format will silently skew per-crate numbers. Extract the dual-prefix condition to a named SQL fragment or helper with a test per branch.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Dual-prefix matching has a dedicated test covering both absolute and relative filename branches
- [ ] #2 Intent of the workspace_root-prefixed join is documented inline at the SQL site
<!-- AC:END -->
