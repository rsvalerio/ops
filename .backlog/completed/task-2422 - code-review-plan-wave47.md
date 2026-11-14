---
id: TASK-2422
title: 'code-review-plan-wave47'
status: Done
assignee: []
created_date: '2026-10-04 14:52'
updated_date: '2026-10-04 16:06'
labels:
  - code-review-wave
dependencies:
  - TASK-2389
  - TASK-2390
  - TASK-2391
  - TASK-2392
  - TASK-2393
  - TASK-2401
  - TASK-2402
  - TASK-2403
  - TASK-2404
  - TASK-2405
modified_files:
  - extensions/config-checkers/src/lib.rs
  - extensions/config-checkers/src/runner.rs
  - extensions/config-checkers/src/tests.rs
  - extensions/config-checkers/src/yaml.rs
  - extensions/create-review-tasks/src/backlog.rs
  - extensions/create-review-tasks/src/lib.rs
  - extensions/hook-common/src/config.rs
  - extensions/hook-common/src/fixtures.rs
  - extensions/hook-common/src/git.rs
  - extensions/hook-common/src/git_state.rs
  - extensions/hook-common/src/install.rs
  - extensions/hook-common/src/lib.rs
  - extensions/run-before-commit/Cargo.toml
  - extensions/text-fixers/src/atomic.rs
  - extensions/text-fixers/src/eof.rs
  - extensions/text-fixers/src/lib.rs
  - extensions/text-fixers/src/runner.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave47
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Hooks, fixers and checkers: text-fixers write-back race, hook-common, run-before-commit, config-checkers and create-review-tasks input validation and docs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: none

Branch: code-review/TASK-2422
Worktree: /home/rsvalerio/projects/.wave-TASK-2422

<!-- SECTION:NOTES:END -->
