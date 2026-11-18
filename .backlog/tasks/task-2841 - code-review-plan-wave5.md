---
id: TASK-2841
title: 'code-review-plan-wave5'
status: To Do
assignee: []
created_date: '2026-11-18 20:32'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-wave
dependencies:
  - TASK-2669
  - TASK-2676
  - TASK-2680
  - TASK-2683
  - TASK-2797
modified_files:
  - crates/core/src/bounded_read.rs
  - crates/core/src/config/edit.rs
  - crates/core/src/config/loader/mod.rs
  - crates/core/src/text.rs
  - crates/core/src/ui.rs
  - extensions/git/src/config.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave5
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Grouping: ops-core bounded reads and sanitising, plus ops-git hardened open
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2861 (extensions/git/src/config.rs)
<!-- SECTION:NOTES:END -->
