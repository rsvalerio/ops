---
id: TASK-2127
title: 'ERR-3: a report write failure fails the whole run after the task set is already durably committed'
status: Done
assignee: []
created_date: '2026-09-08 06:55'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - error-handling
dependencies: []
parent_task_id: 'TASK-2249'
modified_files:
  - extensions/create-review-tasks/src/lib.rs
priority: low
ordinal: 43000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/lib.rs:139` (`run_create_review_tasks_at`), `:469` (`report`)

**What**: `commit_task_set` returns only once every file is on disk and `StagedTasks::keep()` has suppressed the rollback. `report` then runs, and each `writeln!(out, ...)?` propagates an io error straight out of `run_create_review_tasks`. A closed or broken stdout (`ops create-review-tasks | head`, a terminated pager, a full pipe) therefore makes the command exit non-zero even though the review request and all its subtasks were created successfully — and the operator never sees the ids of the tasks that now exist, so the failure looks like "nothing happened" when the truth is the opposite.

**Why it matters**: The error type does not distinguish "the task set could not be created" from "the task set was created but I could not tell you about it", and only the first is a reason to fail the command. The contract in the entry point's `# Errors` section lists neither case. A caller that retries on non-zero exit will allocate a second, duplicate review request. Either the report failure should be reported as a warning over a successful exit, or the error must state plainly that the tasks were created and name the main id so the operator can recover them with `backlog task list --parent`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A stdout write failure after a successful commit no longer presents as an unqualified run failure: either the run succeeds with the write failure surfaced separately, or the error explicitly states the task set was created and names the main task id
- [x] #2 The run_create_review_tasks # Errors documentation covers the report-write case and says what state the backlog is left in
- [x] #3 A test drives a failing writer after a committed set and pins the resulting behaviour
<!-- AC:END -->
