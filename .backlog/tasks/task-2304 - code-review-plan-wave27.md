---
id: TASK-2304
title: 'code-review-plan-wave27'
status: Done
assignee: []
created_date: '2026-09-26 19:55'
updated_date: '2026-09-26 20:39'
labels:
  - code-review-wave
dependencies:
  - TASK-2299
  - TASK-2300
  - TASK-2298
modified_files:
  - extensions/about/src/units.rs
  - extensions-rust/about/src/units.rs
  - extensions-rust/about/src/deps_provider.rs
  - crates/cli/src/args.rs
  - extensions/about/src/machine.rs
  - extensions-rust/cargo-update/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave27
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
ops about follow-ups from wave24: crates --json targets + dev-dep duplicates toggle, machine cfg-table/target-dir resolution, and the spurious cargo-update warn surfaced by about dependencies --duplicates.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2303 (crates/cli/src/args.rs), TASK-2305 (crates/cli/src/args.rs)

Branch: code-review/TASK-2304

<!-- SECTION:NOTES:END -->
