---
id: TASK-0448
title: 'FN-1: run_commands is a 78-line function with two intertwined output modes'
status: To Do
assignee:
  - TASK-0536
created_date: '2026-04-28 05:43'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - FN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:90-168`

**What**: `run_commands` interleaves dry-run dispatch, raw-mode handling (with two warnings), display setup, parallel/sequential branching, and result aggregation in a single function. The `if raw` and post-display blocks contain duplicated success-aggregation logic.

**Why it matters**: Adding a third output mode or flag interaction (raw+tap+dry-run+verbose matrix) requires editing the same large block, raising regression risk.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract run_commands_raw, run_commands_with_display, and a summarize(results) helper paralleling the run_command_raw / run_command_cli single-command split
- [ ] #2 All existing run_cmd tests still pass; new tests cover the raw+tap warning path explicitly
<!-- AC:END -->
