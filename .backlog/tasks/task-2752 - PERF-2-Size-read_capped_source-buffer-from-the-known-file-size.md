---
id: TASK-2752
title: 'PERF-2: Size read_capped_source buffer from the known file size'
status: Done
assignee: []
created_date: '2026-11-18 18:41'
updated_date: '2026-11-18 22:52'
labels:
  - code-review-rust
  - PERF
dependencies: []
parent_task_id: 'TASK-2855'
modified_files:
  - extensions-rust/loc/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'PERF-2:extensions-rust/loc/src/lib.rs:read_capped_source'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/lib.rs:385` (`read_capped_source`), caller `count_entry` at `extensions-rust/loc/src/lib.rs:283`

**What**: `read_capped_source` reads into `Vec::new()` through `reader.take(cap + 1).read_to_end(..)`. The caller already holds the file size from `open_sized` (the same handle's metadata), but the buffer is not pre-sized. `Take<&mut BufReader<File>>` gives `read_to_end` no size hint, so the Vec grows by amortized doubling. A file just under `MAX_SOURCE_BYTES` (4 MiB) can end up with a buffer of about 8 MiB. `String::from_utf8` keeps that capacity, and it stays resident for the whole `count_source` call.

**Why it matters**: The `MAX_SOURCE_BYTES` and `read_capped_source` docs say resident memory is bounded by the cap ("The single extra byte keeps resident memory bounded by the cap"). Doubling growth can make the real per-worker peak about 2x the documented figure. That figure is then multiplied by the parallel walker's worker count. Growing the buffer also costs about log2(n) reallocations and copies per file on the hot scan path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 read_capped_source pre-sizes its buffer from the size already known from open_sized (e.g. Vec::with_capacity of min(size, cap) + 1, converted without as-casts), or takes that size as a parameter
- [x] #2 Docs on MAX_SOURCE_BYTES / read_capped_source accurately describe the per-file resident bound
- [x] #3 Existing read_capped_source_detects_growth_past_the_cap and over-cap tests still pass

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.17 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
