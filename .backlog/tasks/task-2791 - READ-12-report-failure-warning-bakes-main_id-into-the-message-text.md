---
id: TASK-2791
title: 'READ-12: report-failure warning bakes main_id into the message text'
status: Done
assignee: []
created_date: '2026-11-18 18:45'
updated_date: '2026-11-18 21:26'
labels:
  - code-review-rust
  - READ
dependencies: []
parent_task_id: 'TASK-2837'
modified_files:
  - extensions/create-review-tasks/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'READ-12:extensions/create-review-tasks/src/lib.rs:run_create_tasks_at'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/lib.rs:243-249` (`run_create_tasks_at`)

**What**: The warning emitted when the run report cannot be written after commit already records `main_id` as a named field. It then interpolates `plan.main_id` a second time into the message through a positional `{}` (`"... recover the ids with `backlog task list --parent {}`"`). So the message is not a stable template: every occurrence has different text, which defeats grouping by message, and the value is duplicated. The hint also names the external `backlog` binary, though the project directs operators to `ops backlog` (AGENTS.md). The same string appears in `report` at `lib.rs:661`.

**Why it matters**: This is the single event an operator relies on to recover the ids of a committed set whose report was lost. It should be a constant, filterable message with the id in a field, and the command it points to should be the supported one.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The tracing::warn! message is a constant template; main_id is carried only as a named field
- [x] #2 The recovery hint (warning and report line) names ops backlog task list --parent, with tests updated

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.25 (code-review-rust run, 2026-11-18).

Fixed in TASK-2837: warn! message is a constant template with main_id only as a field; warning, report line, docs and tests name ops backlog task list --parent (and the init hint names ops backlog init).

<!-- SECTION:NOTES:END -->
