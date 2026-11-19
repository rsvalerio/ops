---
id: TASK-2774
title: 'PATTERN-9: install_interrupt_flag leaves SIGINT/SIGTERM handlers registered after the run'
status: To Do
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
parent_task_id: 'TASK-2860'
modified_files:
  - extensions/create-coderabbit-tasks/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'PATTERN-9:extensions/create-coderabbit-tasks/src/lib.rs:install_interrupt_flag'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-coderabbit-tasks/src/lib.rs:234-246` (`install_interrupt_flag`)

**What**: `install_interrupt_flag` registers two `signal_hook::flag` handlers per signal (SIGINT, SIGTERM) for process-wide state and discards the returned `SigId`s, so the handlers are never unregistered. Once `run_coderabbit_reviews` returns, the first Ctrl-C or SIGTERM the process receives is still swallowed: it only sets a flag that nothing reads any more. Each further call to `run_coderabbit_reviews` in the same process adds another pair of handlers with a fresh flag.

**Why it matters**: A library function changes process-global signal disposition for the rest of the process's life rather than for the scope that needs it. Today the CLI exits soon after, so the window is small, but any caller that keeps running (a later CLI step, a test harness, a long-lived host) silently loses its first interrupt. The registration is a resource like the worktree and should be released by a guard.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The handler registration is held in a guard whose Drop calls signal_hook::low_level::unregister for every SigId it registered
- [ ] #2 The guard lives exactly as long as the review loop in run_coderabbit_reviews
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.24 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
