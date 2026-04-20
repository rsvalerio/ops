---
id: TASK-0080
title: 'SEC-12: VALUES CTE built by concatenating escaped member paths into SQL'
status: Done
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 14:56'
labels:
  - rust-codereview
  - sec
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query.rs:77`

**What**: prepare_per_crate constructs VALUES (p1), (p2), ... by string-escaping each path rather than using DuckDB parameter binding, even though path values are user/config-derived.

**Why it matters**: Defense-in-depth relies on two validators; any future relaxation could re-open injection. Parameter binding would eliminate the class of bug.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Switch VALUES CTE to parameterized placeholders and bind member paths via duckdb::params
- [ ] #2 Keep validate_path_chars as belt-and-braces but remove escape_sql_string from hot-path SQL
<!-- AC:END -->
