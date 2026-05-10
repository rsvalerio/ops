---
id: TASK-0679
title: 'PERF-2: collect_tools spawns cargo/rustup serially per tool entry'
status: Done
assignee:
  - TASK-0741
created_date: '2026-04-30 05:15'
updated_date: '2026-04-30 19:33'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/lib.rs:81-94`

**What**: `collect_tools` synchronously calls `probe::check_tool_status` for each tool; two cargo/rustup subprocesses spawn per entry serially.

**Why it matters**: With ~10 tools this is ~20 cargo/rustup spawns on a path users hit interactively (`ops tools`). Resolving cargo --list and rustup component list once and reusing the result across all entries would reduce wall-clock time substantially.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Resolve cargo --list and rustup component list --installed once and pass references into check_tool_status
<!-- AC:END -->
