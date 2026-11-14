---
id: TASK-2406
title: 'ERR-9: DbError interpolates its #[from]/#[source] field into the #[error] message'
status: Done
assignee: []
created_date: '2026-10-04 14:18'
updated_date: '2026-10-04 15:28'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2416'
modified_files:
  - extensions/sqlite/src/error.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-9:extensions/sqlite/src/error.rs:DbError'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/error.rs:20-110` (`DbError`)

**What**: Several variants set both the message and the source to the same value: `Sqlite(#[from] rusqlite::Error)` with `#[error("database error: {0}")]`, `Io(#[from] io::Error)` with `"IO error: {0}"`, `Serialization(#[from] serde_json::Error)` with `"serialization error: {0}"`, `SqlValidation(#[from] SqlError)` with `"SQL validation failed: {0}"`, `QueryFailed` with `"{context}: {source}"` (`#[source]`), and `External` with `"external error: {0:#}"`.

**Why it matters**: Every chain-walking printer (anyhow `{:#}`, `{:?}` "Caused by:") prints the cause twice, e.g. `database error: no such table: x: no such table: x`. provide_via_ingestor wraps these in anyhow context, so the duplicated text reaches operators.

<!-- scan confidence: candidates to inspect -->
Candidates: error.rs variants Sqlite, Io, QueryFailed, Serialization, SqlValidation, External.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 No #[error] format string in DbError interpolates the field marked #[from]/#[source]
- [x] #2 Variant messages say what the layer was doing; existing tests asserting on message text are updated

<!-- AC:END -->
