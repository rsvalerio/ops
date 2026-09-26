---
id: TASK-2290
title: 'code-review-plan-wave22'
status: To Do
assignee: []
created_date: '2026-09-26 18:27'
updated_date: '2026-09-26 18:27'
labels:
  - code-review-wave
dependencies:
  - TASK-2278
  - TASK-2279
  - TASK-2280
modified_files:
  - crates/cli/src/main.rs
  - crates/cli/src/backlog_cmd.rs
  - crates/cli/src/run_cmd/plan.rs
  - crates/cli/src/run_cmd/dry_run.rs
  - crates/cli/src/args.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave22
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Dry-run honesty: --dry-run must never execute (hook gates, backlog mutations), plus a read-only ops plan that is tested never to execute.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2291 wave23 (crates/cli/src/main.rs, crates/cli/src/backlog_cmd.rs, crates/cli/src/args.rs); TASK-2293 wave25 (crates/cli/src/args.rs)
<!-- SECTION:NOTES:END -->
