---
id: TASK-2535
title: 'FN-1: run_wave_overlap spans ~79 lines with the scope/overlap computation inline'
status: Done
assignee: []
created_date: '2026-10-10 15:34'
updated_date: '2026-10-10 21:33'
labels:
  - code-review-rust
  - functions
dependencies: []
parent_task_id: 'TASK-2613'
modified_files:
  - crates/backlog/src/cmd/wave.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:crates/backlog/src/cmd/wave.rs:run_wave_overlap'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/wave.rs:222` (run_wave_overlap, ~79 raw lines to line 301)

**What**: run_wave_overlap exceeds the 50-line FN-1 threshold. The selected-vs-compared wave set construction (dedup by ptr equality, lines 234-255) and the per-wave scope/overlap/weight row build (lines 257-294) are two distinct computations living inline in the entry point, ahead of the final render dispatch.

**Why it matters**: The merge-order weighting rule ("least-overlapping first, ties on numeric id") is the algorithm the command exists for, and it is spelled inside a nested iterator chain rather than a named function. Extracting `overlap_rows(entries, selected, compared)` makes the ordering rule testable in isolation.

<!-- scan confidence: raw-line measurement including comments -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The scope/overlap/weight row computation is a named helper; run_wave_overlap stays under ~50 lines
- [x] #2 Row ordering (weight ascending, then numeric id) is unchanged (existing tests pass)

<!-- AC:END -->
