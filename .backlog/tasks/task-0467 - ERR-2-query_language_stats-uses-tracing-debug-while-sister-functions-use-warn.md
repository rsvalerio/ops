---
id: TASK-0467
title: >-
  ERR-2: query_language_stats uses tracing::debug! while sister functions use
  warn
status: To Do
assignee:
  - TASK-0534
created_date: '2026-04-28 05:46'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/code.rs:22`

**What**: query_language_stats calls ctx.get_or_provide("duckdb", data_registry) and on any error other than NotFound logs tracing::debug! then returns None. About-code subpage renders "no language stats" instead of surfacing the failure. Same shape on the "tokei" warm-up. enrich_from_db in lib.rs uses tracing::warn! for the same DB-query-failed-mid-render threat.

**Why it matters**: Inconsistent severity across the about extension: lib.rs warns, units.rs warns, code.rs debugs. A user investigating "why is my about-code empty?" with default tracing config sees nothing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace tracing::debug! with tracing::warn! in query_language_stats for the non-NotFound provider error and the non-empty query_project_languages Err branch, matching the rest of the about crate
- [ ] #2 Doc-test or unit test confirms a non-NotFound provider error in query_language_stats produces a tracing::warn
<!-- AC:END -->
