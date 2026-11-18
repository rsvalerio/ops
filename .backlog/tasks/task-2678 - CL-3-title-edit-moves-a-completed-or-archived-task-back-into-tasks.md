---
id: TASK-2678
title: 'CL-3: title edit moves a completed or archived task back into tasks/'
status: To Do
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - CL
dependencies: []
parent_task_id: 'TASK-2838'
modified_files:
  - crates/backlog/src/cmd/edit.rs
priority: medium
ordinal: 1000
dedup_key: 'CL-3:crates/backlog/src/cmd/edit.rs:rename_to_new_slug'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/edit.rs:170-221` (`rename_to_new_slug`)

**What**: `run_edit` resolves the task with `Store::find`, which searches `tasks/` → `completed/` → `archive/tasks/`. When `-t/--title` changes the slug, `rename_to_new_slug` builds the destination with `store.task_path(&file_name)`, which is hard-wired to `<root>/tasks/`, instead of `old_path.with_file_name(..)`. The task is linked into `tasks/` and the original in `completed/`/`archive/` removed — the implicit assumption is that every editable task lives in `tasks/`.

Reproduced on ops 0.94.1: create TASK-0001, move its file to `.backlog/completed/`, run `ops backlog task edit TASK-0001 -t renamed` → the file now lives at `.backlog/tasks/task-0001 - renamed.md`.

**Why it matters**: A cosmetic rename silently resurrects a cleaned-up task into the live set (it reappears in `task list`, `search`, readiness and wave scans) and can collide with a live task of the same id, since `completed/`/`archive/` ids are not unique.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A title edit renames the file within the directory it already lives in (completed/ and archive/tasks/ included)
- [ ] #2 A test edits the title of a task in completed/ and asserts the file stays in completed/ under the new slug and nothing appears in tasks/
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.01 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
