---
id: TASK-0102
title: code-review-plan-wave7
status: To Do
assignee:
  - code-review-wave
created_date: '2026-04-17 12:07'
labels:
  - code-review-wave
dependencies:
  - TASK-0066
  - TASK-0067
  - TASK-0068
  - TASK-0073
  - TASK-0082
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave7
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Config error handling: stop swallowing load_config/current_dir/parse errors via unwrap_or_default, surface diagnostics, switch eager context to lazy with_context.
<!-- SECTION:PLAN:END -->
