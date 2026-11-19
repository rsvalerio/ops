---
id: TASK-2759
title: 'TEST-11: query_coverage_files_round_trip asserts field presence, not values'
status: To Do
assignee: []
created_date: '2026-11-18 18:41'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2854'
modified_files:
  - extensions-rust/test-coverage/src/tests/provider.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-11:extensions-rust/test-coverage/src/tests/provider.rs:query_coverage_files_round_trip'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests/provider.rs:55-84`

**What**: `query_coverage_files_round_trip` checks the two filenames, then for every row asserts only `row.get("<column>").is_some()` for all 15 columns. No numeric value read back from `coverage_files` is compared against the fixture (`sample_coverage_json`: e.g. src/lib.rs lines_count 200, regions_notcovered 2, branches_percent 90.0).

**Why it matters**: `query_coverage_files` (src/provider.rs) builds each `CoverageRow` by hand from 15 `row.get` calls, and its doc claims a swapped or mis-bound column is caught. This test would pass if `regions_covered` and `regions_notcovered` were swapped, if a percent column were read from the wrong field, or if every value were 0 — the round trip it is named for is not verified.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The test compares at least one full row (all 15 fields) to the fixture values for that filename
- [ ] #2 Swapping two same-typed columns in query_coverage_files makes the test fail
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.19 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
