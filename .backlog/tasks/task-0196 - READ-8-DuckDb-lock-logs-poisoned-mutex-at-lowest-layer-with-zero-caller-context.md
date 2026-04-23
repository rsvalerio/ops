---
id: TASK-0196
title: >-
  READ-8: DuckDb::lock logs poisoned mutex at lowest layer with zero caller
  context
status: To Do
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/connection.rs:77-82`

**What**: DuckDb::lock emits tracing warn on poison, then returns DbError::MutexPoisoned. Other modules (enrich_from_db in about) swallow the error without re-logging, so a poisoned mutex logs exactly once at the lowest layer without caller context.

**Why it matters**: READ-8/ERR-1 — log at handling site, not inside a library primitive. Move the warn to callers that decide to drop the error; keep DuckDb::lock quiet. Related to the separate ERR-1 finding on enrich_from_db.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DuckDb::lock no longer logs; callers that swallow the error log with context
- [ ] #2 Poison-path log carries the query/label that failed
<!-- AC:END -->
