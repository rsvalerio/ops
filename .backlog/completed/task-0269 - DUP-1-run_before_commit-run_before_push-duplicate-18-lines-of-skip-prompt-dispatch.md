---
id: TASK-0269
title: >-
  DUP-1: run_before_commit/run_before_push duplicate 18+ lines of
  skip/prompt/dispatch
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 07:49'
labels:
  - rust-code-review
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:101`

**What**: Bodies differ only by hook name, SKIP_ENV_VAR, has_staged_files predicate.

**Why it matters**: Diverges over time; CLI-side counterpart to existing hook-dup task.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Extract run_hook helper parameterized by HookOps
- [x] #2 Invoke from both wrappers
<!-- AC:END -->
