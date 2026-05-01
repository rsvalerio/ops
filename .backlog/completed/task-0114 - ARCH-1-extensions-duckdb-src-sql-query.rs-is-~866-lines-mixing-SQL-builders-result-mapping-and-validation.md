---
id: TASK-0114
title: >-
  ARCH-1: extensions/duckdb/src/sql/query.rs is ~866 lines mixing SQL builders,
  result mapping, and validation
status: Done
assignee: []
created_date: '2026-04-19 18:41'
updated_date: '2026-04-19 19:20'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/query.rs:1-866`

**What**: Single module contains multiple query builders (LOC, deps, coverage, crate metadata), row-to-struct mapping, identifier validation, and error construction in ~866 lines.

**Why it matters**: Large SQL-building modules are high-risk surfaces (injection, regressions across queries). Splitting per query family (loc, deps, coverage) and separating validation helpers would shrink blast radius and make per-query testing tractable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 query.rs is split into focused submodules (e.g. loc, deps, coverage) with file sizes under ~400 lines each
- [x] #2 public API surface from extensions/duckdb/src/sql is unchanged (no behavior regression)
<!-- AC:END -->
