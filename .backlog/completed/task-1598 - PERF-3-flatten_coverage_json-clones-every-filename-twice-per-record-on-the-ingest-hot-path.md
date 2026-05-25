---
id: TASK-1598
title: >-
  PERF-3: flatten_coverage_json clones every filename twice per record on the
  ingest hot path
status: Done
assignee:
  - TASK-1634
created_date: '2026-05-21 22:53'
updated_date: '2026-05-22 08:36'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:138-177`

**What**: Each record path performs `filename.to_string()` inside `CoverageRow::from_summary` (line 46) and then `record.filename.clone()` inside `dedup_push` (line 167) to build the `HashMap<String, usize>` key. Two heap allocations per file for a value already owned by the record; the `HashMap` retains a third duplicate `String`.

**Why it matters**: This is the per-row hot path of ingest. The intent of `with_capacity(total)` + the `Entry` cleanup in TASK-1558 was throughput; cloning the key here defeats it. On a 5k-file workspace this is ~10k unnecessary `String` allocations + drops per ingest. Complements TASK-1558 (Done).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Restructure dedup_push so the hash key borrows from records[idx].filename — e.g., switch to HashMap<String, CoverageRow> with .into_values().collect() at the end, or use the raw_entry_mut API
- [ ] #2 Confirm per-record String allocation count drops from 3 to 1 (bench or instrumented count)
- [ ] #3 Last-write-wins ordering preserved (verified by flatten_coverage_json_dedups_overlapping_filenames_across_exports)
<!-- AC:END -->
