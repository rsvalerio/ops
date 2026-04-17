---
id: TASK-0069
title: 'DUP-1: tokio Runtime::new and block_on scaffolding duplicated in run_cmd'
status: To Do
assignee: []
created_date: '2026-04-17 11:30'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - dup
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:86`

**What**: `Runtime::new()?` is constructed in both run_commands (L86) and run_command_cli (L277), each time a command runs; this also duplicates the block_on scaffolding.

**Why it matters**: Creating a fresh multi-threaded runtime per invocation is wasteful and couples CLI dispatch to runtime lifecycle.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract a single helper (e.g. run_with_runtime) that owns the Runtime and takes an async closure
- [ ] #2 Both run_commands and run_command_cli delegate to the helper
<!-- AC:END -->
