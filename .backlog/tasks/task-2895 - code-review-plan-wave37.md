---
id: TASK-2895
title: 'code-review-plan-wave37'
status: To Do
assignee: []
created_date: '2026-11-20 13:26'
updated_date: '2026-11-20 13:26'
labels:
  - code-review-wave
dependencies:
  - TASK-2888
modified_files:
  - crates/core/src/subprocess/spawn.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave37
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Subprocess spawn: keep unlocked spawns from overlapping captured spawns on non-pipe2 targets.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: none
<!-- SECTION:NOTES:END -->
