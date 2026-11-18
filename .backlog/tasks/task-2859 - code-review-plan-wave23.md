---
id: TASK-2859
title: 'code-review-plan-wave23'
status: To Do
assignee: []
created_date: '2026-11-18 20:32'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-wave
dependencies:
  - TASK-2773
  - TASK-2776
  - TASK-2782
  - TASK-2822
  - TASK-2824
  - TASK-2826
  - TASK-2827
  - TASK-2828
modified_files:
  - crates/cli/src/doc_ids_cmd.rs
  - crates/cli/src/subcommands.rs
  - extensions/config-checkers/src/json.rs
  - extensions/config-checkers/src/lib.rs
  - extensions/config-checkers/src/runner.rs
  - extensions/config-checkers/src/tests.rs
  - extensions/config-checkers/src/yaml.rs
  - extensions/text-fixers/src/atomic.rs
  - extensions/text-fixers/src/discovery.rs
  - extensions/text-fixers/src/lib.rs
  - extensions/text-fixers/src/options.rs
  - extensions/text-fixers/src/runner.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave23
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Grouping: config-checkers and text-fixers: shared discovery, atomic replace, options API
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2847 (extensions/config-checkers/src/lib.rs); TASK-2858 (crates/cli/src/subcommands.rs)
<!-- SECTION:NOTES:END -->
