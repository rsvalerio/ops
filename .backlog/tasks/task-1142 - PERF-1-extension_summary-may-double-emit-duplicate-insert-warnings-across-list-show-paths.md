---
id: TASK-1142
title: >-
  PERF-1: extension_summary may double-emit duplicate-insert warnings across
  list/show paths
status: To Do
assignee:
  - TASK-1263
created_date: '2026-05-08 07:41'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:128-162`

**What**: The fallback branch (no static `command_names`) calls `ext.register_commands(&mut local)` and surfaces `take_duplicate_inserts` warnings. `print_extension_details` (line 272) calls `extension_summary(ext)` independently of the table render path that already called it via line 105. For a self-shadowing extension, an operator running `ops extension show foo` may double-warn.

**Why it matters**: PERF-1/TASK-0859 hoisted the per-row loop fix; the cross-handler duplication is the same shape one level up. Operator-facing duplicate warning undermines the audit signal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cache extension_summary result for the duration of a CLI invocation, or have print_extension_details pass through a precomputed summary
- [ ] #2 Add a test that asserts a single warn per self-shadow per CLI invocation
<!-- AC:END -->
