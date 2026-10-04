---
id: TASK-2414
title: 'ERR-2: tokei scan silently drops files deeper than ScanLimits::depth with no skip count or warning'
status: Triage
assignee: []
created_date: '2026-10-04 14:19'
labels:
  - code-review-rust
  - ERR
dependencies: []
modified_files:
  - extensions/tokei/src/lib.rs
  - extensions/tokei/src/tests.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-2:extensions/tokei/src/lib.rs:collect_candidates'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/tokei/src/lib.rs:~352` (`collect_candidates`, `WalkBuilder::max_depth`), `TokeiScan`, `collect_tokei`

**What**: `WalkBuilder::max_depth(Some(limits.depth))` stops descent at depth 32 without recording it. Oversize, unreadable and file-cap truncation each feed `Skips` and the "statistics are incomplete" warning, but files below the depth cap are neither counted nor reported, so `collect_tokei` returns a short count that looks complete. `scan_tokei_honours_the_depth_cap` locks in only the silent drop.

**Why it matters**: the module's own stated goal is that a short answer is distinguishable from a correct one (ERR-2, TASK-1972). Depth is the one bound that breaks it. Low impact: real trees rarely exceed 32 levels.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A walk that hits the depth cap is surfaced (for example a Skips/TokeiScan depth_truncated flag) and included in the incomplete-statistics warning
- [ ] #2 The depth-cap test asserts the new signal
<!-- AC:END -->
