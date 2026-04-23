---
id: TASK-0134
title: 'ERR-4: .ok()? discards PoisonError / query errors in code.rs'
status: To Do
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - err
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/code.rs:31` (db lock) and `extensions/about/src/code.rs:52` (stmt.query_map)

**What**: `db.lock().ok()?` and `stmt.query_map(...).ok()?` collapse meaningful errors (PoisonError, SQL error) into `None`, losing diagnostic context.

**Why it matters**: Failures show up as silent absence of data, not as actionable errors; debugging a poisoned mutex or malformed SQL requires the original error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace .ok()? with Result-based propagation (?, map_err + context) so PoisonError/duckdb::Error surfaces to the caller
- [ ] #2 Log or propagate context describing what was attempted (lock acquisition, query)
<!-- AC:END -->
