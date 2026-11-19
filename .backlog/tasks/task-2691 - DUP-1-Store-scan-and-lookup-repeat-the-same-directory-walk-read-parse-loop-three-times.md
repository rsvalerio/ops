---
id: TASK-2691
title: 'DUP-1: Store scan and lookup repeat the same directory walk + read + parse loop three times'
status: Done
assignee: []
created_date: '2026-11-18 18:27'
updated_date: '2026-11-18 21:41'
labels:
  - code-review-rust
  - DUP
dependencies: []
parent_task_id: 'TASK-2838'
modified_files:
  - crates/backlog/src/store.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-1:crates/backlog/src/store.rs:Store::scan_all_tasks'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/store.rs:130-149` (`scan_tasks`), `crates/backlog/src/store.rs:168-194` (`scan_all_tasks`), `crates/backlog/src/store.rs:211-238` (`find`)

**What**: All three methods carry the same block: `read_existing_dir` → for each entry `readable_entry(..).path()` → skip non-`.md` → `read_to_string` with `reading {path}` error → `TaskDoc::parse` with `parsing {path}` error. `scan_tasks` and `scan_all_tasks` differ only in which directories they visit and whether they attach a `TaskLocation`; the per-directory sort is duplicated too. `find` repeats it with a tolerant (skip-on-error) variant. `walk_task_files` already exists as the shared walker for names but not for parsed docs.

**Why it matters**: Error wording, filtering (e.g. the `.md` rule) and future fixes (a parse-cache, a filename pre-filter) have to be applied in three places; they have already started to diverge (find tolerates unreadable files, the scans do not), which is intended but invisible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 One private helper reads and parses the .md entries of a directory (with a strict/tolerant mode), used by scan_tasks, scan_all_tasks and find
- [x] #2 Existing store tests pass unchanged

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.01 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
