---
id: TASK-2431
title: 'READ-13: remaining ops-sqlite docs narrate history outside the files TASK-2408 covered'
status: Triage
assignee: []
created_date: '2026-10-04 15:28'
labels:
  - code-review-rust
  - READ
dependencies: []
modified_files:
  - extensions/sqlite/src/connection.rs
  - extensions/sqlite/src/error.rs
  - extensions/sqlite/src/sql/query/helpers.rs
  - extensions/sqlite/src/sql/query/coverage.rs
  - extensions/sqlite/src/sql/ingest/orchestrator.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/connection.rs` (`mint_db_id`, `Sqlite::id`, `ingest_locks`, `open_readonly`), `extensions/sqlite/src/error.rs` (`DbError` container comment, `MutexPoisoned`, `NotFileBacked`), `extensions/sqlite/src/sql/query/helpers.rs` (`query_project_row`, `prepare_per_crate` comments), `extensions/sqlite/src/sql/query/coverage.rs` (`query_project_coverage`), `extensions/sqlite/src/sql/ingest/orchestrator.rs`, plus test doc comments across the crate

**What**: TASK-2408 rewrote the docs in dir.rs, sql.rs, validation.rs, ingestor.rs, sidecar.rs and schema.rs. The files above still carry journey text: "the prior pointer-address scheme had", "once got committed to this repository", "the former hand-rolled query_project_coverage", "The previous iter().map(...)", "now flow through the same scaffolding", and rule/TASK prefixes on most items.

**Why it matters**: Same as TASK-2408: history belongs in commits, and it goes stale while looking authoritative.

**Origin**: discovered during TASK-2416 while fixing TASK-2408.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Docs in the listed files describe current behaviour and invariants only
<!-- AC:END -->
