---
id: TASK-2305
title: 'code-review-plan-wave28'
status: To Do
assignee: []
created_date: '2026-09-26 19:55'
updated_date: '2026-09-26 19:55'
labels:
  - code-review-wave
dependencies:
  - TASK-2296
  - TASK-2294
modified_files:
  - crates/backlog/src/cmd/wave.rs
  - crates/cli/src/args.rs
  - crates/cli/src/backlog_cmd.rs
  - crates/backlog/src/store.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave28
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
backlog crate follow-ups: wave create --members one-step wave creation, and a compile gate for the non-Unix Store::lock_allocation branch.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2303 (crates/cli/src/args.rs), TASK-2304 (crates/cli/src/args.rs)
<!-- SECTION:NOTES:END -->
