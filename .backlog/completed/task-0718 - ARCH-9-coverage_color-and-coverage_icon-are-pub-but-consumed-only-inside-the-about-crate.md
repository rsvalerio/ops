---
id: TASK-0718
title: >-
  ARCH-9: coverage_color and coverage_icon are pub but consumed only inside the
  about crate
status: Done
assignee: []
created_date: '2026-04-30 05:31'
updated_date: '2026-05-02 09:01'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/coverage.rs:35` and `:44`

**What**: `coverage_icon(pct: f64) -> &str` and `coverage_color(pct: f64) -> Color` are `pub fn` items in `extensions/about/src/coverage.rs`, but neither is re-exported from `lib.rs` and neither is referenced outside the `coverage` module (the only other call sites are `format_coverage_table` and `format_coverage_section` in this same file).

**Why it matters**: ARCH-9 (minimal public surface): a `pub` symbol is part of the crate API. Once external code starts depending on `coverage_color` it becomes a breaking-change point. Demoting these helpers to `pub(crate)` (or `pub(super)`) keeps the crate API to the documented `run_about_coverage*` entry points and the `PROJECT_COVERAGE_PROVIDER` constant, leaving the icon/color decision an internal rendering detail.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Demote coverage_icon and coverage_color to pub(crate) (or pub(super)) so they stay internal
- [x] #2 Verify nothing outside extensions/about consumes them
- [x] #3 Keep CoverageTier private if it is also unused outside this module
<!-- AC:END -->
