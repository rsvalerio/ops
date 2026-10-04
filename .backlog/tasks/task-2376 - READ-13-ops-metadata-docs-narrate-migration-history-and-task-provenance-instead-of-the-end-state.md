---
id: TASK-2376
title: 'READ-13: ops-metadata docs narrate migration history and task provenance instead of the end state'
status: To Do
assignee: []
created_date: '2026-10-04 14:13'
updated_date: '2026-10-04 14:52'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2419'
modified_files:
  - extensions-rust/metadata/src/lib.rs
  - extensions-rust/metadata/src/views.rs
  - extensions-rust/metadata/src/ingestor.rs
  - extensions-rust/metadata/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions-rust/metadata/src:crate-docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:2,50-70,84-100,386-400`, `extensions-rust/metadata/src/views.rs:19-60`, `extensions-rust/metadata/src/ingestor.rs:73-92,160-175`, `extensions-rust/metadata/src/tests.rs:1-15`

**What**: Module and item docs/comments describe how the code got here rather than what it is. Examples: the `METADATA_MAX_BYTES_CEILING` doc ("The historical value (`u32::MAX`) is kept from the `DuckDB` era so existing deployments ... see no behavior change"), `CAP_GUARD_SQL` ("SQLite port note"), `crate_dependencies_view_sql` ("SQLite JSON1 port of the `DuckDB` `unnest` body, verified against ... `docs/duckdb-alternatives.md`"), `build_views` ("closed by construction under the SQLite port"), `validate_published` ("failures that used to leak past them"), and `tests.rs` (a changelog of the old 1280-line file, still saying "`DuckDB` payload-cap behaviour" although the backend is SQLite). Dozens of comments also carry task-ID breadcrumbs (`TASK-2188`, `TASK-1893`, ...) and rule IDs as provenance (roughly 60 `TASK-nnnn` references across the crate).

**Why it matters**: The text is meaningless to someone using or maintaining the crate, and it goes stale while looking authoritative (the `tests.rs` header already names the wrong engine). It buries the real invariants (singleton row, SEC-25 anchoring, cap semantics) in narrative.

<!-- scan confidence: candidates to inspect -->
Candidates (grep `DuckDB|duckdb|port|used to|TASK-`): lib.rs, views.rs, ingestor.rs, tests.rs, tests/payload_cap.rs, tests/wiring.rs, test_support.rs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Docs and comments in ops-metadata describe current behaviour only: no DuckDB/port/migration narrative and no 'used to' history
- [ ] #2 Task-ID breadcrumbs are removed from doc comments; design rationale that is still load-bearing is kept as a plain statement of the invariant
- [ ] #3 tests.rs module doc no longer references DuckDB or the old single-file layout
<!-- AC:END -->
