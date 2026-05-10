---
id: TASK-0727
title: 'PATTERN-1: coverage_tier classifies NaN as High via cascading <-comparisons'
status: Done
assignee:
  - TASK-0738
created_date: '2026-04-30 05:48'
updated_date: '2026-04-30 18:40'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/coverage.rs:24-32`

**What**: `coverage_tier(pct: f64)` uses `if pct < 50.0 { Low } else if pct < 80.0 { Medium } else { High }`. For `f64::NAN`, every `<` comparison returns `false`, so a NaN coverage percentage falls through to `CoverageTier::High`, producing a green check icon and Color::Green — the most reassuring possible UI outcome for what is in fact a malformed input. Knock-on: `coverage_icon` (line 36) and `coverage_color` (line 44) inherit the misclassification.

**Why it matters**: NaN can reach this code through DuckDB SUM divisions on empty/zero-line tables (`NULLIF` is used in `query_project_languages` but not consistently), through corrupted/handwritten coverage JSON (`lines_percent: NaN`), or through future arithmetic on f64 fields. A NaN classifying as "High" silently lies to the user about coverage health. The fix is either explicit `pct.is_nan()` rejection (returning Low or a dedicated Unknown tier) or `partial_cmp` based threshold checks that surface NaN.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 coverage_tier handles NaN explicitly (Low or new Unknown tier) with a regression test
- [ ] #2 coverage_icon and coverage_color do not return the High variant for NaN inputs
<!-- AC:END -->
