---
id: TASK-2656
title: 'code-review-plan-wave79'
status: Done
assignee: []
created_date: '2026-11-01 10:31'
updated_date: '2026-11-01 13:47'
labels:
  - code-review-wave
dependencies:
  - TASK-2649
  - TASK-2653
modified_files:
  - crates/core/src/config/commands.rs
  - crates/runner/src/command/resolve.rs
  - docs/configuration.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave79
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Rationale: how [commands] entries resolve from .ops.toml, both follow-ups from TASK-2644. Naming the running ops binary without PATH (2649), and reporting unknown after names on plan-less commands (2653).
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2655/wave78 (docs/configuration.md)

Branch: code-review/TASK-2656
Worktree: /home/rsvalerio/projects/.wave-TASK-2656

<!-- SECTION:NOTES:END -->
