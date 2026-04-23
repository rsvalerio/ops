---
id: TASK-0184
title: 'CONC-2: load_with_sidecar holds Mutex guard across std::fs sidecar read'
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - CONC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/ingestor.rs:69-94`

**What**: load_with_sidecar acquires conn = db.lock()? and keeps the guard held while executing two SQL statements, a COUNT query, and reading the workspace sidecar via read_workspace_sidecar which performs synchronous file I/O before drop(conn) at line 94. Slow disk during sidecar read blocks every other DuckDB caller.

**Why it matters**: CONC-2 variant for sync Mutex — hold locks briefly. Sidecar read can be moved before db.lock() since it does not depend on the connection.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 read_workspace_sidecar is invoked before acquiring the db lock or after dropping it
- [ ] #2 Critical section of load_with_sidecar is restricted to execute/query_row calls
<!-- AC:END -->
