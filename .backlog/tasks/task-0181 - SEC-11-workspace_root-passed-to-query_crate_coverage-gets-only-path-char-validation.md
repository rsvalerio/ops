---
id: TASK-0181
title: >-
  SEC-11: workspace_root passed to query_crate_coverage gets only path-char
  validation
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - SEC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/coverage.rs:45`, `extensions/duckdb/src/sql/validation.rs:54-69`

**What**: validate_path_chars is called on workspace_root but validate_no_traversal is not. workspace_root flows into a bound parameter so SQL injection is not reachable, but the validation layering is inconsistent with prepare_path_for_sql which chains both checks plus escape.

**Why it matters**: SEC-11/SEC-14 — input validation gap. Impact limited since parameterized (no injection) but semantics: a traversal-style workspace_root can produce nonsense matches.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 workspace_root validation is consistent with prepare_path_for_sql or explicitly documented as sufficient given parameterization
- [ ] #2 Validation helpers document which are safe standalone vs in combination
<!-- AC:END -->
