---
id: TASK-2788
title: 'SEC-25: review-request task files are visible empty or half-written to concurrent backlog scans'
status: Done
assignee: []
created_date: '2026-11-18 18:45'
updated_date: '2026-11-18 21:26'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2837'
modified_files:
  - extensions/create-review-tasks/src/lib.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-25:extensions/create-review-tasks/src/lib.rs:stage_task_file'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/lib.rs:558-586` (`stage_task_file`)

**What**: `stage_task_file` creates each task file in place with `File::create_new`, which leaves a zero-byte file at its final path, and only then renders the frontmatter into it through a `BufWriter`. A concurrent `ops_backlog::store::Store::scan_tasks` call (`ops backlog task list`/`search`, or the dedup check of `task create --unless-exists`) parses every file in `tasks/`. If it reads the file during that window, `TaskDoc::parse` errors with `parsing <path>: ...` and the whole scan fails. The rollback path makes it worse: the main file and earlier subtasks are visible, then deleted, while later subtasks are still being written. The write is also not fsynced before `staged.keep()` commits the set, so after a crash the set can be on disk with empty files, and every later scan then fails until someone removes them by hand.

`ops-backlog` already has a stage-then-link no-clobber writer (`atomic_write_noclobber`, `crates/backlog/src/cmd/mod.rs`) that publishes a complete file atomically while keeping `create_new`'s refusal to overwrite.

**Why it matters**: Concurrent review agents filing tasks with `ops backlog task create --unless-exists` fail at random while a review request is being written. A crash part-way through leaves the backlog unreadable for every scanning command.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each task file is rendered to a staging path and published to its final name atomically with no-clobber semantics (e.g. reuse ops-backlog's atomic_write_noclobber), so a scan never sees an empty or partial file
- [x] #2 Taken-name detection (AlreadyExists leads to reallocation) and the all-or-nothing rollback still hold, and their existing tests still pass
- [x] #3 A test asserts that no file under .backlog/tasks is empty or unparseable at any point a concurrent scan can observe (or at least after a failed attempt's rollback)

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.25 (code-review-rust run, 2026-11-18).

Fixed in TASK-2837: stage_task_file renders in memory and publishes via ops_backlog::cmd::atomic_write_noclobber (now pub); DestinationTaken maps to reallocation; render_task_file returns String. Test: concurrent_scans_never_see_a_partial_task_file; rollback test asserts no staging leftovers.

<!-- SECTION:NOTES:END -->
