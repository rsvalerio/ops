---
id: TASK-2644
title: 'code-review-plan-wave75'
status: Done
assignee: []
created_date: '2026-10-31 23:55'
updated_date: '2026-11-01 00:35'
labels:
  - code-review-wave
dependencies:
  - TASK-2637
  - TASK-2641
modified_files:
  - crates/core/src/config/commands.rs
  - crates/core/src/config/clone.rs
  - crates/core/src/config/loader/mod.rs
  - crates/runner/src/command/parallel.rs
  - crates/runner/src/command/resolve.rs
  - crates/runner/src/command/build.rs
  - crates/runner/src/command/tests/parallel.rs
  - crates/cli/src/run_cmd/explain.rs
  - crates/cli/src/run_cmd/plan.rs
  - crates/cli/src/dry_run.rs
  - docs/configuration.md
  - docs/commands.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave75
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
New step/command fields (after, wrap) that change how a plan resolves and runs and must both surface in ops explain and dry-run. Same schema, loader validation, runner and explain code. TASK-2641 needs a recorded decision on composite-level service lifetime.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2645 (crates/cli/src/dry_run.rs, crates/core/src/config/commands.rs, docs/commands.md, docs/configuration.md); TASK-2646 (docs/configuration.md)

Branch: code-review/TASK-2644
Worktree: /home/rsvalerio/projects/.wave-TASK-2644

<!-- SECTION:NOTES:END -->
