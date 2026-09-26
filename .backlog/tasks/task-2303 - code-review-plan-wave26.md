---
id: TASK-2303
title: 'code-review-plan-wave26'
status: To Do
assignee: []
created_date: '2026-09-26 19:55'
updated_date: '2026-09-26 19:55'
labels:
  - code-review-wave
dependencies:
  - TASK-2301
  - TASK-2297
modified_files:
  - crates/cli/src/main.rs
  - crates/cli/src/subcommands.rs
  - crates/cli/src/lock_cmd.rs
  - crates/cli/src/args.rs
  - crates/core/src/.default.terraform.ops.toml
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave26
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
CLI builtin subcommand semantics: every builtin must honour or refuse global --dry-run, and no stack default may be shadowed by a builtin. Both want a test enumerating the clap builtin set; land together.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2304 (crates/cli/src/args.rs), TASK-2305 (crates/cli/src/args.rs)
<!-- SECTION:NOTES:END -->
