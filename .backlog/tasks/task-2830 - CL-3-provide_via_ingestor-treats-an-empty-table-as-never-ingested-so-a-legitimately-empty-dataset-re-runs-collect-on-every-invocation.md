---
id: TASK-2830
title: 'CL-3: provide_via_ingestor treats an empty table as never ingested, so a legitimately empty dataset re-runs collect on every invocation'
status: Done
assignee: []
created_date: '2026-11-18 18:50'
updated_date: '2026-11-18 21:47'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2853'
modified_files:
  - extensions/sqlite/src/sql/ingest/orchestrator.rs
  - extensions/sqlite/src/sql/ingest/sql.rs
priority: medium
ordinal: 1000
dedup_key: 'CL-3:extensions/sqlite/src/sql/ingest/orchestrator.rs:provide_via_ingestor'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/sqlite/src/sql/ingest/orchestrator.rs:123-148` (`provide_via_ingestor`), `extensions/sqlite/src/sql/ingest/sql.rs:432-449` (`table_has_data`)

**What**: The orchestrator's only signal for "this source is already loaded" is `table_has_data`, which returns `row_count > 0`. A collector that legitimately produces zero records still loads successfully and writes its `data_sources` provenance row. Examples are tokei on a tree with no recognised source files, or a coverage run whose report lists no files. The table it creates is empty, though, so every later `provide_via_ingestor` call sees `has_data == false` and re-runs `ingestor.collect` (for test-coverage, the whole instrumented test run) and `load`. The `data_sources` row, the durable record that the load happened, is never consulted. The precondition "a loaded source always has at least one row" is implicit and undocumented.

**Why it matters**: An unbounded, repeated, expensive re-collection happens on every `ops` invocation for any workspace whose dataset is empty. Nothing tells the operator about it: the rows are still empty, just slowly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 provide_via_ingestor decides whether to collect from a load marker that distinguishes 'loaded, zero rows' from 'never loaded' (e.g. the data_sources row for the source, or table existence plus provenance), not from row_count > 0
- [x] #2 A test with an ingestor that stages an empty array asserts collect runs once across two provide_via_ingestor calls

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.30 (code-review-rust run, 2026-11-18).

Fixed in TASK-2853: provide_via_ingestor probes source_is_loaded (table exists AND a data_sources row exists for ingestor.name()) instead of table_has_data. Test: empty_dataset_is_collected_once_across_calls.

<!-- SECTION:NOTES:END -->
