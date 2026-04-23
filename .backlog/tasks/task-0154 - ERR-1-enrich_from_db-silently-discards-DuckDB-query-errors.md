---
id: TASK-0154
title: 'ERR-1: enrich_from_db silently discards DuckDB query errors'
status: To Do
assignee: []
created_date: '2026-04-22 21:22'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**: `extensions/about/src/lib.rs:132-158`, `extensions/about/src/units.rs:60-63`

**What**: `enrich_from_db` uses `if let Ok(...)` and `.unwrap_or_default()` / `.ok()` to absorb every query error from `query_project_loc`, `query_project_file_count`, `query_dependency_count`, `query_project_coverage`, `query_project_languages`, `query_crate_loc`, `query_crate_file_count`. No tracing, no warning — a broken schema, poisoned mutex, or malformed data surfaces as a silent "no data available" in the about card.

**Why it matters**: ERR-1/SEC-31 anti-pattern "swallowed errors". Regressions in the DuckDB layer become invisible. Fix: emit `tracing::warn!` (or debug) with context when queries fail; keep the graceful default only after logging.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 enrich_from_db logs failed queries via tracing with source-error context
- [ ] #2 a unit test asserts the logging path for at least one failing query
<!-- AC:END -->
