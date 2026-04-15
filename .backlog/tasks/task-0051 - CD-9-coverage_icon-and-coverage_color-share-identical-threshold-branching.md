---
id: TASK-0051
title: 'CD-9: coverage_icon and coverage_color share identical threshold branching'
status: Done
assignee: []
created_date: '2026-04-14 20:31'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-4
  - DUP-5
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `extensions-rust/about/src/format.rs:117-137`
**Anchor**: `fn coverage_icon`, `fn coverage_color`
**Impact**: Both functions branch on the same thresholds (`< 50.0`, `< 80.0`, else) returning different types (`&str` emoji vs `Color` enum). If thresholds change, both must be updated in lockstep. A single `coverage_tier(pct) -> CoverageTier` enum (e.g., Low/Medium/High) consumed by both would eliminate the parallel branching.

DUP-4: identical match arms within the same module. DUP-5: extract shared threshold logic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Threshold logic defined once; coverage_icon and coverage_color derive output from a shared tier
<!-- AC:END -->
