---
id: TASK-0194
title: 'DUP-1: PerCrateSetup destructuring duplicated in coverage.rs and helpers.rs'
status: To Do
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query/coverage.rs:48-58`, `extensions/duckdb/src/sql/query/helpers.rs:195-202`

**What**: Both query_crate_coverage and query_per_crate_i64 destructure PerCrateSetup with the same three-arm match: Empty to empty map, NoTable to zeroed map from member_paths, Ready to continue. The pattern is inlined at both call sites rather than hoisted into a helper.

**Why it matters**: DUP-1 — identical 6-line match across two files. A future change to NoTable zeroing will need both sites updated.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 PerCrateSetup unwrapping is factored into a single helper
- [ ] #2 coverage.rs and helpers.rs call the shared helper
<!-- AC:END -->
