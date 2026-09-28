---
id: TASK-2340
title: 'code-review-plan-wave36'
status: Done
assignee: []
created_date: '2026-09-28 16:38'
updated_date: '2026-09-28 17:03'
labels:
  - code-review-wave
dependencies:
  - TASK-2336
  - TASK-2335
modified_files:
  - extensions-rust/deps/src/lib.rs
  - crates/cli/src/run_cmd/tools.rs
  - docs/commands.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave36
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Rationale: deps extension command registration and ops explain tool reporting; both surfaced in TASK-2333 and share extensions-rust/deps/src/lib.rs (2336 moves deps to ExecCommandSpec::ops_subcommand; 2335 adds versions to explain tools[]).
Overlaps: TASK-2334/wave35 (docs/commands.md)

Branch: code-review/TASK-2340

<!-- SECTION:NOTES:END -->
