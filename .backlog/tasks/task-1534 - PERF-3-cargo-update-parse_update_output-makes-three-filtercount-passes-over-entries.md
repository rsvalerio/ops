---
id: TASK-1534
title: >-
  PERF-3: cargo-update parse_update_output makes three filter+count passes over
  entries
status: Done
assignee:
  - TASK-1575
created_date: '2026-05-19 09:53'
updated_date: '2026-05-19 17:08'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:139-150`

**What**: After building `entries`, `parse_update_output` walks the same `Vec<UpdateEntry>` three separate times via `entries.iter().filter(...).count()` — once for each of `update_count`, `add_count`, `remove_count`. A single fold (or maintaining the counters during the parse loop) covers the same work in one pass.

**Why it matters**: This sits on the data-provider hot path (`ops about --refresh` invokes it every run) and the same struct is the marshalled output, so the counts are computed every time. The walks are O(n) each and produce identical answers; consolidating to one pass — or accumulating counters while we push entries in the loop above — removes two redundant iterations without changing behaviour.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Counts are computed in a single pass over entries (either accumulated during the parse loop or via a single fold).
- [ ] #2 All existing tests still pass; counts match the previous filter+count behaviour for Update/Add/Remove entries.
<!-- AC:END -->
