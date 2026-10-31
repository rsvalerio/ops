---
id: TASK-2645
title: 'code-review-plan-wave76'
status: To Do
assignee: []
created_date: '2026-10-31 23:55'
updated_date: '2026-10-31 23:55'
labels:
  - code-review-wave
dependencies:
  - TASK-2639
  - TASK-2640
modified_files:
  - crates/core/src/.default.rust.ops.toml
  - crates/runner/src/command/builtins.rs
  - crates/cli/src/args.rs
  - crates/cli/src/main.rs
  - crates/cli/src/help.rs
  - crates/cli/src/dry_run.rs
  - crates/cli/src/subcommands.rs
  - crates/cli/src/run_cmd/tools.rs
  - crates/core/src/config/commands.rs
  - crates/core/src/config/merge.rs
  - docs/commands.md
  - docs/configuration.md
  - docs/command-mappings.md
  - docs/foundation.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave76
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Removes repetition from Rust repos on the foundation: stack built-ins (semver, features, dep-floors, cog-check, doc-ids, foundation) replacing per-repo scripts and definitions, plus the default-category decision. Same built-in registration path as ops msrv.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2644 (crates/cli/src/dry_run.rs, crates/core/src/config/commands.rs, docs/commands.md, docs/configuration.md); TASK-2646 (crates/cli/src/args.rs, crates/cli/src/main.rs, docs/configuration.md, docs/foundation.md)
<!-- SECTION:NOTES:END -->
