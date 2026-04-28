---
id: TASK-0513
title: >-
  PERF-1: extension_summary re-runs register_commands for every list/show
  invocation
status: To Do
assignee:
  - TASK-0536
created_date: '2026-04-28 06:51'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/extension_cmd.rs:100`

**What**: extension_summary builds a fresh CommandRegistry and calls ext.register_commands for every list and show invocation, just to read command names.

**Why it matters**: Each call repeats the registration work the runtime already performs once at startup; for an extension that performs heavy I/O during register_commands (e.g. tools provider) the help/list path duplicates that cost.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Cache the per-extension command name list, or expose a lightweight command_names() accessor
- [ ] #2 list/show paths avoid re-registering
<!-- AC:END -->
