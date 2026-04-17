---
id: TASK-0096
title: >-
  FN-1: query_loc_data in about/query.rs exceeds 50 lines and mixes provider
  init, two query layers, and assembly
status: To Do
assignee: []
created_date: '2026-04-17 11:55'
updated_date: '2026-04-17 12:07'
labels:
  - rust-code-review
  - complexity
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs` (fn `query_loc_data`, ~65 lines)

**What**: `query_loc_data` opens with two `ctx.get_or_provide(...)` calls (duckdb/tokei) that each `tracing::debug!` and return `None` on error, then resolves the db, then issues four DuckDB queries (`query_project_loc`, `query_project_file_count`, `query_crate_loc`, `query_crate_file_count`) each with its own `match ... tracing::debug! ... return None/continue` ladder, and finally assembles `LocData`. Four repeated error-branch blocks and two responsibilities (setup + query orchestration) live in the same function.

**Why it matters**: Violates FN-1 (single abstraction level) and FN-2 (readability of early-return ladders). Any change to the error-handling policy touches four near-identical blocks (DUP-2). Adding a fifth datum (e.g. test count) forces another ~10 lines of boilerplate inside the same function.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 query_loc_data body is <=30 lines and operates at a single abstraction level
- [ ] #2 Provider bootstrap (get_or_provide for duckdb + tokei) is a named helper that returns Result<(), ()> or logs once
- [ ] #3 Per-crate query boilerplate (match + tracing::debug + unwrap_or_default) is deduplicated into a helper (e.g. try_query_or_log) used by both query_crate_loc and query_crate_file_count
- [ ] #4 Existing tests for the about rust identity flow still pass with no behavior change
<!-- AC:END -->
