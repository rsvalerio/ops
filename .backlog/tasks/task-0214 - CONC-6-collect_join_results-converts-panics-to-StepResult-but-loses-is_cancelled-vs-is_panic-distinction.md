---
id: TASK-0214
title: >-
  CONC-6: collect_join_results converts panics to StepResult but loses
  is_cancelled vs is_panic distinction
status: To Do
assignee: []
created_date: '2026-04-23 06:32'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - concurrency
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:453`

**What**: Match arm formats `task panicked: {}` regardless of whether JoinError is a panic or a cancellation from JoinSet abort.

**Why it matters**: When fail_fast aborts tasks, cancelled tasks are reported as panics to the user, which is misleading for diagnostics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Branch on e.is_cancelled() and emit a skipped/cancelled StepResult instead of a panicked one
- [ ] #2 Add a test that aborts a JoinSet and asserts the resulting StepResult message reflects cancellation
<!-- AC:END -->
