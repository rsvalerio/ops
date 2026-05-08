---
id: TASK-1138
title: >-
  ASYNC-7: CLI selects MultiThread runtime for any_parallel even when leaf count
  is 1
status: To Do
assignee:
  - TASK-1271
created_date: '2026-05-08 07:40'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - ASYNC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:230-249`

**What**: `run_commands_with_display` selects `RuntimeKind::MultiThread` whenever `plan.any_parallel` is true, even for `command_ids.len() <= 1` where `run_plan_parallel` itself shortcuts to `run_plan` (parallel.rs:289-291). For a one-command plan that happens to be a `parallel = true` composite expanding to a single leaf, the CLI pays multi-thread runtime startup.

**Why it matters**: TASK-0875's stated goal was eliminating `worker_thread × CPU` spin-up for sequential CLI invocations; the parallel-flag check happens upstream of the leaf-count check that actually decides whether parallelism runs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Decide runtime kind on plan.leaf_ids.len() > 1 && plan.any_parallel
- [ ] #2 Or document the asymmetry on RuntimeKind so a 1-leaf parallel plan still gets multi-thread
<!-- AC:END -->
