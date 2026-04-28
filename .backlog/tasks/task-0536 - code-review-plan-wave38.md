---
id: TASK-0536
title: code-review-plan-wave38
status: In Progress
assignee:
  - code-review-wave
created_date: '2026-04-28 07:13'
updated_date: '2026-04-28 16:24'
labels:
  - code-review-wave
dependencies:
  - TASK-0425
  - TASK-0426
  - TASK-0427
  - TASK-0444
  - TASK-0445
  - TASK-0446
  - TASK-0447
  - TASK-0448
  - TASK-0511
  - TASK-0512
  - TASK-0513
  - TASK-0514
  - TASK-0529
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Wave 38 — crates/cli: init/theme/run/extension/help commands, command registry, .ops.toml loading, logging init.
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Wave-38 run completed except TASK-0427 (config-load threading), which was deferred back to To Do for re-triage as a dedicated wave: scope is ~10 handler signatures across run_cmd/subcommands/extension_cmd/theme_cmd/tools_cmd/about_cmd/hook_shared plus a non-trivial load-count regression test design. ops verify and ops qa pass clean.
<!-- SECTION:NOTES:END -->
