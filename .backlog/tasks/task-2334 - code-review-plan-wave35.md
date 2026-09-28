---
id: TASK-2334
title: 'code-review-plan-wave35'
status: To Do
assignee: []
created_date: '2026-09-28 15:07'
updated_date: '2026-09-28 15:07'
labels:
  - code-review-wave
dependencies:
  - TASK-2327
  - TASK-2328
modified_files:
  - crates/cli/src/args.rs
  - crates/cli/src/subcommands.rs
  - crates/cli/src/main.rs
  - .github/workflows/ci.yml
  - docs/commands.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave35
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rationale: replace ci.yml's hand-rolled MSRV and workflow-guard jobs with ops built-ins (msrv, lint-actions). forge-side replacement (2328 AC#2) is out of repo. Scope is predicted (feature tasks carry no --modified-file); new source files may be added.
Overlaps: TASK-2331/wave32 (crates/cli/src/args.rs, crates/cli/src/subcommands.rs, .github/workflows/ci.yml, docs/commands.md); TASK-2333/wave34 (crates/cli/src/args.rs, docs/commands.md)
<!-- SECTION:NOTES:END -->
