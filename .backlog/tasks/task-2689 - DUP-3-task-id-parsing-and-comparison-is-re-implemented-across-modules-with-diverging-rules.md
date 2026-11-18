---
id: TASK-2689
title: 'DUP-3: task-id parsing and comparison is re-implemented across modules with diverging rules'
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
  - crates/backlog/src/cmd/wave.rs
  - crates/backlog/src/cmd/list.rs
  - crates/backlog/src/cmd/edit.rs
  - crates/backlog/src/cmd/commit.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-3:crates/backlog/src/store.rs:task_id_key'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/store.rs:320-385` (`leading_task_number_of`, `task_id_key`, `leading_task_number`), `crates/backlog/src/cmd/wave.rs:170-180,235-263,332-342,635-640` (`is_member`, `select_waves`, `id_order`, `looks_like_task_id`), `crates/backlog/src/cmd/list.rs:80-94` (`parent_matches`, `dependents_match`), `crates/backlog/src/cmd/edit.rs:226-230` (`extract_file_id`), `crates/backlog/src/cmd/commit.rs:219-228` (`WantedId`)
<!-- scan confidence: candidates to inspect -->

**What**: There are at least six independent id parsers/comparators with different rules: `leading_task_number_of` splits at the *first* `-`, `task_id_key` at the *last* `-`, `leading_task_number` requires a literal lowercase `task-`, `id_order` uses `rsplit_once('-')` and parses the dotted tail as u64 (so `TASK-12.03` has no numeric key), `looks_like_task_id` rejects dotted subtask ids. `Store::find` and `commit` accept bare/padded forms via `task_id_key`, but `wave members`/`wave overlap`/`list --parent`/`list --dependents`/`wave create` compare raw strings with `eq_ignore_ascii_case`, so `TASK-12` vs `TASK-0012` or `1764.02` match in some commands and not in others.

**Why it matters**: Each new command picks a different matching rule; users see an id accepted by `view` rejected by `wave overlap`, and dependency/parent links written in a non-canonical form are silently ignored by membership queries.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 One TaskId parsing/comparison helper (e.g. a TaskId newtype or task_id_key-based eq) owns the matching rule
- [x] #2 wave members/overlap, list --parent/--dependents and wave create match ids with that helper, so padded and bare forms resolve consistently with Store::find

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.01 (code-review-rust run, 2026-11-18).

TaskFileName::parse keeps its own leading_task_number for id allocation (conservative, filename-only, documented); every id comparison goes through store::same_task_id / task_id_lookup_key.

<!-- SECTION:NOTES:END -->
