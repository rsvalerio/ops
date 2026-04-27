---
id: TASK-0370
title: >-
  ARCH-2: about/code.rs duplicates per-language SQL already covered by
  sql::query::query_project_languages
status: To Do
assignee:
  - TASK-0420
created_date: '2026-04-26 09:37'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/code.rs:47`

**What**: query_language_stats reimplements the language aggregation query inline (SELECT language, SUM(code), COUNT(*) ... GROUP BY language ORDER BY loc DESC) instead of calling the existing ops_duckdb::sql::query_project_languages.

**Why it matters**: Two parallel implementations drift apart over time (already do — one omits percentages, applies different filtering); also forces re-doing lock-and-decode boilerplate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the inline statement with a call to query_project_languages (or a sibling helper) returning shared types
- [ ] #2 Inline LanguageStat struct removed in favor of ops_core::project_identity::LanguageStat
<!-- AC:END -->
