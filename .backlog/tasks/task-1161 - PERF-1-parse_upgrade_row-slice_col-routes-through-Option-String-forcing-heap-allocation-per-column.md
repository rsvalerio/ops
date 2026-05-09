---
id: TASK-1161
title: >-
  PERF-1: parse_upgrade_row::slice_col routes through Option<String> forcing
  heap allocation per column
status: Done
assignee:
  - TASK-1263
created_date: '2026-05-08 07:45'
updated_date: '2026-05-09 11:06'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse.rs:198`

**What**: `parse_upgrade_row::slice_col` does `slice.trim().to_string()` per column per row, then immediately consumed as the `String` field of `UpgradeEntry`. Six allocations per row × N rows. Sister hot-path optimisation `contains_ascii_ci` was already added; the row builder itself wasn't. Closure form deliberately routes through `Option<String>` forcing heap allocation even when the trimmed slice is empty.

**Why it matters**: cargo upgrade --dry-run output is small in practice (<200 rows), so this is Low. Alternative parses borrowed from `line` and only `to_string()` once at the `UpgradeEntry` construction site.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Refactor slice_col to return Option<&str>; do the to_string() in the final UpgradeEntry literal
- [x] #2 Net effect: 0 allocations in empty-trim path; 5–6 allocations per real row instead of 5–6 plus intermediate Option<String>
<!-- AC:END -->
