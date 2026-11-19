---
id: TASK-2865
title: 'SEC-25: cleanup moves task files to completed/ outside the allocation lock, racing task edit'
status: Done
assignee: []
created_date: '2026-11-18 21:21'
updated_date: '2026-11-19 17:28'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2879'
modified_files:
  - crates/backlog/src/cmd/cleanup.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/cleanup.rs:250` (`move_to_completed`)

**What**: `task edit` now holds `Store::lock_allocation()` across find → edit → write (TASK-2682). `cleanup` still moves Done tasks into `completed/` (link the new name, unlink the old) without the lock. An edit that resolved the task in `tasks/` just before the move then `atomic_write`s back to the old `tasks/` path after the unlink, resurrecting the task in `tasks/` with the new content while the stale copy sits in `completed/` — two files carrying one id.

**Why it matters**: Duplicate ids break `find` (first match wins), wave membership and `ops backlog commit`; cleanup is run while agents may still be editing tasks.

**Origin**: discovered during TASK-2837 while fixing TASK-2682.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 cleanup holds Store::lock_allocation() across the scan and each move, so a concurrent task edit either sees the task in completed/ or finishes before it moves
- [x] #2 A test interleaves an edit with a cleanup move and asserts exactly one file carries the id afterwards

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
cleanup re-scans and moves under lock_allocation after confirmation, refusing if the selection changed. Tests: edit_before_the_move_rides_into_completed, edit_that_changes_the_selection_moves_nothing, racing_edit_and_cleanup_leave_one_file_with_the_note.
<!-- SECTION:NOTES:END -->
