---
id: TASK-1282
title: 'DUP-1: run_before_commit and run_before_push duplicate dispatch shape'
status: Done
assignee:
  - TASK-1305
created_date: '2026-05-11 15:26'
updated_date: '2026-05-11 18:18'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:181-207`

**What**: `run_before_commit` and `run_before_push` share an identical structural pattern: match Install action -> call `*_install(&config)?; Ok(SUCCESS)`, else dispatch to `run_hook_dispatch(config, &CONST_OPS, flag)`. The two functions differ only in (a) the action enum type, (b) the install fn pointer, (c) the HookOps constant, and (d) whether `changed_only` is forwarded or hard-coded false.

**Why it matters**: A new hook (e.g. run-before-merge) means duplicating this shape a third time. The differences are exactly what `HookOps` was designed to abstract (per the TASK-0757 comment at line 140), but the dispatch wrappers were not folded in.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either HookOps gains an install_fn: fn(&Config) -> Result<()> and a changed_only_supported: bool, so a single run_hook(config, hook, action_is_install, changed_only) replaces both wrappers; or Install variants are normalised into a tiny shared HookInstallAction so one match handles both
- [ ] #2 Existing parse and dispatch tests still pass
- [ ] #3 Cross-cut with API-1 finding on run_before_push so changed_only flow is normalised in the same refactor
<!-- AC:END -->
