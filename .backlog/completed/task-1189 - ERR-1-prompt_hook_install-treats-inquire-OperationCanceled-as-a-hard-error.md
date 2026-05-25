---
id: TASK-1189
title: 'ERR-1: prompt_hook_install treats inquire OperationCanceled as a hard error'
status: Done
assignee:
  - TASK-1268
created_date: '2026-05-08 08:12'
updated_date: '2026-05-10 06:30'
labels:
  - code-review-rust
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/subcommands.rs:99`

**What**: When inquire::Confirm returns Err (e.g. user hits Ctrl-C), the ? propagates an anyhow error up. The surrounding flow reports ExitCode::FAILURE for an interrupted prompt that the user explicitly cancelled — there's no distinction between cancel and real failure in the exit path.

**Why it matters**: A user pressing Ctrl-C at the install prompt sees `ops: error: <inquire ctrl-c message>` and a non-zero exit, blocking the git operation that triggered the dispatch. A cancelled-by-user condition should bail cleanly without the angry "ops: error:" framing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 prompt_hook_install distinguishes inquire::InquireError::OperationCanceled / OperationInterrupted from real errors and returns a clean exit (or 130) without the ops-error decoration.
- [ ] #2 Regression test using a stub Confirm that returns OperationCanceled asserts no 'ops: error:' is emitted.
<!-- AC:END -->
