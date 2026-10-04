---
id: TASK-2408
title: 'READ-13: Docs in ops-sqlite narrate migration history and task journeys'
status: Triage
assignee: []
created_date: '2026-10-04 14:18'
labels:
  - code-review-rust
  - READ
dependencies: []
modified_files:
  - extensions/sqlite/src/sql/ingest/dir.rs
  - extensions/sqlite/src/sql/ingest/sql.rs
  - extensions/sqlite/src/sql/validation.rs
  - extensions/sqlite/src/ingestor.rs
  - extensions/sqlite/src/sql/ingest/sidecar.rs
  - extensions/sqlite/src/schema.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:extensions/sqlite/src/sql/ingest/dir.rs:module docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/ingest/dir.rs:55-90,265-300,590-610` (`create_ingest_dir`, `IngestDir`, `checksum_reader`, `external_err`), `extensions/sqlite/src/sql/ingest/sql.rs:1-12,150-165` (module docs, `JsonTableLoad`), `extensions/sqlite/src/sql/validation.rs:24-28`, `extensions/sqlite/src/ingestor.rs:100-175`, `extensions/sqlite/src/sql/ingest/sidecar.rs:60-75`, `extensions/sqlite/src/schema.rs:75-90`

**What**: Doc comments describe how the code got here rather than its end state: "closed by the SQLite port", "replaces the DuckDB read_json_auto builder", "Before this type existed...", "TASK-2039 weighed two answers and took the cheaper one", "the path-based checksum_file that used to share this core is gone", "this section used to name a DbError::SidecarTooLarge that does not exist", "the old io_err", plus the "Decision:" and "A future hardening (option B in TASK-1008)" passages. Nearly every item carries a rule/TASK prefix.

**Why it matters**: A reader of the API does not need this history, and it goes stale (see the separate READ-4 task on `load_with_sidecar`). It belongs in commit messages or an ADR. Keep the enduring invariants (anchored descriptor, bound parameters, why parent hardening and anchoring are complementary).

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Item and module docs describe current behaviour and invariants only; migration narrative, 'used to'/'old'/'previously' passages and design-decision essays are removed
- [ ] #2 Rule/TASK ids are not required in doc text
<!-- AC:END -->
