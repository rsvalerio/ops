---
id: TASK-2670
title: 'SEC-25: task create hands out duplicate ids under concurrency and exposes half-written task files'
status: To Do
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2837'
modified_files:
  - crates/backlog/src/cmd/create.rs
  - crates/backlog/src/store.rs
priority: high
ordinal: 1000
dedup_key: 'SEC-25:crates/backlog/src/cmd/create.rs:create_task'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/create.rs:113-156` (`create_task`), `crates/backlog/src/store.rs:253-263` (`next_task_number`), `crates/backlog/src/store.rs:474-476` (`main_task_file_name`)

**What**: Id allocation is check-then-act. `create_task` scans for the highest number (`next_task_number`), derives `task-<n> - <slug>.md`, and relies on `File::create_new` returning `AlreadyExists` to detect a race. But the path includes the title slug, so two concurrent creates with *different* titles derive different paths for the same number and both succeed. The allocation lock is only taken when `--unless-exists` is set, so unkeyed creates (and a keyed create racing an unkeyed one) are not serialized at all.

Reproduced on ops 0.94.1: 12 parallel `ops backlog task create "title $i"` in a fresh backlog produced `task-0002 - title-5.md` + `task-0002 - title-9.md` and `task-0008 - title-4.md` + `task-0008 - title-8.md` — two tasks per id.

Second defect in the same function: the file is created empty (`create_new`) and then filled with `write_all`, with no staging/rename and no fsync. A concurrent `scan_tasks` (list, search, a keyed create's dedup check) can read the zero-byte or partial file; `TaskDoc::parse` rejects it (`empty file: expected ---`) and the whole scan errors. A failed `write_all` (ENOSPC) leaves the truncated file behind permanently, after which every `scan_tasks` call fails. The crate already has `atomic_write_noclobber` (`cmd/mod.rs:219`) that stages + hard-links, but `create_task` does not use it.

**Why it matters**: Duplicate ids silently break `find` (first match wins), wave membership, dependencies and `commit`. Concurrent review agents filing tasks is the documented use case of this crate (code-review skills run in parallel).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Concurrent creates with different titles never produce two task files carrying the same id (e.g. allocation lock always held around scan+claim, or the number claimed independent of the slug)
- [ ] #2 A new task file is never observable empty or partially written: content is staged and published atomically with no-clobber semantics
- [ ] #3 A regression test spawns N concurrent unkeyed creates with distinct titles and asserts N distinct ids
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.01 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
