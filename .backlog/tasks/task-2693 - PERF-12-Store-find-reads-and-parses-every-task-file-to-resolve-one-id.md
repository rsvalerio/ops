---
id: TASK-2693
title: 'PERF-12: Store::find reads and parses every task file to resolve one id'
status: To Do
assignee: []
created_date: '2026-11-18 18:28'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - PERF
dependencies: []
parent_task_id: 'TASK-2838'
modified_files:
  - crates/backlog/src/store.rs
priority: low
ordinal: 1000
dedup_key: 'PERF-12:crates/backlog/src/store.rs:Store::find'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/store.rs:211-238` (`Store::find`); callers `crates/backlog/src/cmd/view.rs:34-37` (`run_view`), `crates/backlog/src/cmd/commit.rs:188-214` (`resolve_task_paths`)

**What**: `find` walks `tasks/`, `completed/` and `archive/tasks/` and does `read_to_string` + full `TaskDoc::parse` on every `.md` file until the id matches — the filename already carries the id (`task-<n>[.<mm>] - <slug>.md`, see `TaskFileName::parse`), but it is not used to skip non-candidates. On this repo's ~2.7k task files a lookup of a completed task parses thousands of documents. `run_view` then calls `scan_tasks` and parses all of `tasks/` again; `commit` calls `find` once per id (O(ids × files)).

**Why it matters**: Every `task view`/`edit`/`commit`/`wave claim` pays a full-tree parse; the cost grows with the backlog's history and is paid repeatedly by agents in loops.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 find pre-filters entries by the id parsed from the filename (falling back to a full parse only for names without a derivable id) before reading/parsing
- [ ] #2 Lookup semantics (precedence order, case/padding tolerance, damaged-neighbour tolerance) are unchanged; existing store tests pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.01 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
