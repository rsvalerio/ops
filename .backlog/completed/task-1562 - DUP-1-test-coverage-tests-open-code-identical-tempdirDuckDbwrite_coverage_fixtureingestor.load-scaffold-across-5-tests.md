---
id: TASK-1562
title: >-
  DUP-1: test-coverage tests open-code identical
  tempdir+DuckDb+write_coverage_fixture+ingestor.load scaffold across 5+ tests
status: Done
assignee:
  - TASK-1577
created_date: '2026-05-19 15:51'
updated_date: '2026-05-19 18:05'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests.rs:186,226,597,640,705`

**What**: The same 5-line scaffold

```rust
let data_dir = tempfile::tempdir().expect("tempdir");
let db = DuckDb::open_in_memory().expect("open in-memory db");
write_coverage_fixture(data_dir.path());
let ingestor = CoverageIngestor;
let _ = ingestor.load(data_dir.path(), &db).expect("load");
```

is duplicated across `coverage_load_creates_table_and_view`, `coverage_summary_view_computes_percentages`, `query_coverage_files_round_trip`, `coverage_summary_view_all_metric_percentages`, and `coverage_load_is_idempotent`. A `setup_loaded_db()` helper returning `(TempDir, DuckDb)` collapses the boilerplate and keeps each test focused on the assertion it owns.

Additionally, `ingestor.rs:54-93` (`coverage_load_with_sample_data`) re-builds the same coverage fixture from scratch instead of sharing `write_coverage_fixture` — same shape, different module.

**Why it matters**: When the fixture shape or the load contract changes (e.g. new sidecar required, schema bump), every duplicate must be updated by hand. Today the fixture column list is in `tests.rs::sample_coverage_json`, `ingestor.rs::coverage_load_with_sample_data`, and `tests.rs::coverage_summary_view_handles_zero_counts` — three places already drifting in details (number of files, column subsets).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Introduce a single test helper that builds a TempDir + DuckDb + loaded fixture; reuse from all currently duplicating tests in tests.rs
- [ ] #2 Move the bespoke setup in ingestor.rs::coverage_load_with_sample_data onto the shared fixture helper (or document why it must diverge)
- [ ] #3 Run cargo test -p ops-test-coverage and confirm no behaviour change
<!-- AC:END -->
