---
id: TASK-1553
title: >-
  FN-1: flatten_coverage_json spans 106 lines combining export iteration,
  filename validation, section extraction, and dedup bookkeeping
status: Done
assignee:
  - TASK-1577
created_date: '2026-05-19 15:34'
updated_date: '2026-05-19 18:05'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:210-316`

**What**: `flatten_coverage_json` is 106 lines and interleaves four distinct concerns:

1. Validate `data` array shape and capture per-export `files` arrays (lines 211-239).
2. Iterate every export and validate `filename` (lines 253-271).
3. Build a 15-field `serde_json::json!` record per file via `extract_section` calls (lines 272-295).
4. Maintain a `filename → records-slot` map for last-write-wins dedup with a duplicate counter and warn (lines 250-314).

**Why it matters**: A function this size with this many concerns is the exact shape FN-1 targets: contributors editing the dedup logic must re-read the parsing logic, and the per-record construction (15 keys) sits in the middle of control flow it does not belong to. Splitting `build_record(filename, summary) -> serde_json::Value` and `dedup_push(records, idx_map, record, &mut dup_count)` would let the outer function read as a straight loop over `file_arrays` plus a final warn. The body is exercised by ~10 unit tests already, so the refactor is mechanical.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 flatten_coverage_json body <= 50 lines (FN-1 threshold)
- [ ] #2 Per-file record construction extracted into a private helper that takes filename + summary
- [ ] #3 Existing tests in src/tests.rs continue to pass with no semantic change
<!-- AC:END -->
