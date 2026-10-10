---
id: TASK-2624
title: 'code-review-plan-wave70'
status: To Do
assignee: []
created_date: '2026-10-10 21:12'
updated_date: '2026-10-10 21:13'
labels:
  - code-review-wave
dependencies:
  - TASK-2551
  - TASK-2553
  - TASK-2556
  - TASK-2558
  - TASK-2559
  - TASK-2560
  - TASK-2561
  - TASK-2569
  - TASK-2570
  - TASK-2571
  - TASK-2572
  - TASK-2573
  - TASK-2574
  - TASK-2575
  - TASK-2577
modified_files:
  - extensions/config-checkers/src/error.rs
  - extensions/git/Cargo.toml
  - extensions/git/src/remote.rs
  - extensions/hook-common/Cargo.toml
  - extensions/hook-common/src/config.rs
  - extensions/hook-common/src/git.rs
  - extensions/hook-common/src/git_state.rs
  - extensions/hook-common/src/install.rs
  - extensions/hook-common/src/lib.rs
  - extensions/run-before-commit/src/lib.rs
  - extensions/run-before-push/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave70
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Hook subsystem (hook-common, run-before-commit, run-before-push, git, config-checkers): provenance and rule-ID stripping, long-function split, test dedup, forbid unsafe
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2608 (extensions/git/Cargo.toml)
<!-- SECTION:NOTES:END -->
