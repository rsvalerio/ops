---
id: TASK-2354
title: 'ERR-4: Task scans silently drop directory-entry read errors, unlike Store::find'
status: Done
assignee: []
created_date: '2026-10-04 14:08'
updated_date: '2026-10-04 15:49'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2423'
modified_files:
  - crates/backlog/src/store.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-4:crates/backlog/src/store.rs:scan_tasks'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/store.rs:136` (`scan_tasks`), `store.rs:187` (`scan_all_tasks`), `store.rs:394-428` (`for_each_task_file`, `find_task_file`)

**What**: These iterate `read_dir` with `.flatten()`, discarding per-entry `io::Error`s. `Store::find` deliberately surfaces the same failure ("a partial scan must not read as not found"), so the three scanning paths are inconsistent. `for_each_task_file` / `find_task_file` also treat an unreadable directory as absent; they feed `next_number` id allocation, so an unreadable `completed/` or `archive/` directory can make the allocator reuse an existing id.

**Why it matters**: A transient EIO or permission problem yields an under-counted listing (`list`, `about`, `cleanup`) or a duplicate task id with no diagnostic.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 scan_tasks and scan_all_tasks return an error naming the directory when an entry read fails, matching Store::find
- [x] #2 Id allocation surfaces (or explicitly tests the documented decision for) an unreadable task directory instead of silently skipping it

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
ops backlog's own allocator (Store::next_task_number) now returns an error naming the unreadable directory. The public free walkers for_each_task_file / find_task_file keep their tolerant no-error-channel contract (now documented and pinned by a test) because their only callers live in extensions/create-review-tasks, outside this wave's file scope; migrating that allocator is filed as a separate Triage task.
<!-- SECTION:NOTES:END -->
