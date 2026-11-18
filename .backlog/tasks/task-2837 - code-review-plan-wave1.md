---
id: TASK-2837
title: 'code-review-plan-wave1'
status: To Do
assignee: []
created_date: '2026-11-18 20:32'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-wave
dependencies:
  - TASK-2670
  - TASK-2682
  - TASK-2692
  - TASK-2673
  - TASK-2685
  - TASK-2786
  - TASK-2788
  - TASK-2791
modified_files:
  - crates/backlog/src/cmd/cleanup.rs
  - crates/backlog/src/cmd/create.rs
  - crates/backlog/src/cmd/edit.rs
  - crates/backlog/src/cmd/mod.rs
  - crates/backlog/src/model.rs
  - crates/backlog/src/store.rs
  - extensions/create-review-tasks/src/backlog.rs
  - extensions/create-review-tasks/src/lib.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave1
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Grouping: backlog: id allocation, write atomicity and frontmatter integrity (TASK-2670 race family)
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2838 (crates/backlog/src/cmd/create.rs, crates/backlog/src/cmd/edit.rs, crates/backlog/src/store.rs)
<!-- SECTION:NOTES:END -->
