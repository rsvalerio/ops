---
id: TASK-0449
title: >-
  DUP-1: run_before_commit_cmd and run_before_push_cmd are near-identical
  74-line modules
status: Done
assignee:
  - TASK-0535
created_date: '2026-04-28 05:43'
updated_date: '2026-04-28 13:58'
labels:
  - code-review-rust
  - DUP
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_before_commit_cmd.rs` and `crates/cli/src/run_before_push_cmd.rs`

**What**: The two hook command modules differ only in `HookOps` constants and a small handful of strings. The dispatcher, install path, and tests are line-for-line analogues.

**Why it matters**: Adding a new pre-* hook (e.g. `pre-rebase`) requires copying the whole file again; stable invitation to drift in error messages, test naming, install flow.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Collapse into a single pre_hook_cmd.rs module parameterised by HookOps (already the abstraction boundary), or generate the two modules from a macro
- [x] #2 All hook install / dispatch tests still pass
<!-- AC:END -->
