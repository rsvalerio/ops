---
id: TASK-1538
title: >-
  DUP-3: BufWriter+MakeWriter tracing-capture scaffold duplicated 2x inside
  metadata/src/tests.rs
status: Done
assignee:
  - TASK-1576
created_date: '2026-05-19 15:23'
updated_date: '2026-05-19 17:48'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/tests.rs:1091-1110, 1189-1208`

**What**: The `BufWriter` newtype + `MakeWriter` impl block is defined twice inside `tests.rs` — once inside `metadata_package_index_by_id_warns_on_duplicate_id` (lines 1091-1110) and again, byte-identical, inside `metadata_package_index_by_name_warns_on_duplicate_name` (lines 1189-1208). Each copy is ~20 lines covering: `BufWriter(Arc<Mutex<Vec<u8>>>)`, `impl Write` (write/flush), `impl<'a> MakeWriter<'a> for BufWriter` with `make_writer = self.clone()`. The pair is a verbatim duplicate of the pattern already centralised by TASK-1157 / TASK-1311 / TASK-1429 / TASK-1494, just open-coded in this crate.

**Why it matters**: A future change to the harness (e.g. switching to `ops_about::test_support::TracingBuf`, which is already imported in `ingestor.rs` tests at line 392) has to be applied in two places inside this file before drift sets in. The sister test `metadata_load_warns_when_metadata_raw_has_multiple_rows` (ingestor.rs:391) already uses `TracingBuf::default()` — the two tests in `tests.rs` should adopt the same helper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Both duplicate-warning tests in metadata/src/tests.rs use a single shared tracing-capture helper (e.g. ops_about::test_support::TracingBuf) instead of two open-coded BufWriter/MakeWriter blocks
- [ ] #2 No #[derive(Clone, Default)] struct BufWriter remains inside metadata/src/tests.rs
- [ ] #3 Both tests still observe exactly one warn line per duplicate after consolidation
<!-- AC:END -->
