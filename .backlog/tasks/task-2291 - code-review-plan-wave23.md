---
id: TASK-2291
title: 'code-review-plan-wave23'
status: To Do
assignee: []
created_date: '2026-09-26 18:27'
updated_date: '2026-09-26 18:27'
labels:
  - code-review-wave
dependencies:
  - TASK-2281
  - TASK-2285
  - TASK-2286
  - TASK-2289
modified_files:
  - crates/cli/src/args.rs
  - crates/cli/src/main.rs
  - crates/backlog/src/cmd/wave.rs
  - crates/backlog/src/render.rs
  - crates/backlog/src/cmd/mod.rs
  - crates/cli/src/backlog_cmd.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave23
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Wave-runner support: ops lock, wave overlap, backlog commit, wave claim/park — replace the shell in code-review-run-wave(s) and triage.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2290 wave22 (crates/cli/src/main.rs, crates/cli/src/backlog_cmd.rs, crates/cli/src/args.rs); TASK-2293 wave25 (crates/cli/src/args.rs)
<!-- SECTION:NOTES:END -->
