---
id: TASK-0866
title: >-
  FN-3: run_commands_with_display has #[allow(clippy::too_many_arguments)]
  hiding boolean-arg footgun
status: Done
assignee: []
created_date: '2026-05-02 09:21'
updated_date: '2026-05-02 10:46'
labels:
  - code-review-rust
  - complexity
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:173-181`

**What**: The function takes six positional parameters (runner, leaf_ids, any_parallel, fail_fast, tap, verbose) and silences clippy with #[allow(clippy::too_many_arguments)]. Two of the six are bools in adjacent positions (any_parallel, fail_fast), inviting the same swap bugs that motivated RunOptions for the public entry point.

**Why it matters**: FN-3 explicitly recommends grouping into config structs; run_commands_raw shares four of the six arguments and would benefit from a shared PlanExecutionParams. The #[allow] is the smell.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Group the four plan-shape parameters into a struct (e.g. PlanShape { leaf_ids, any_parallel, fail_fast }) or extend RunOptions
- [ ] #2 Drop the #[allow(clippy::too_many_arguments)] after the refactor
- [ ] #3 Both run_commands_raw and run_commands_with_display thread the same struct for symmetry
<!-- AC:END -->
