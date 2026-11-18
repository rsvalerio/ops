---
id: TASK-2690
title: 'TIME-6: create/edit/wave commands read the wall clock inside the handler instead of taking it from the edge'
status: Done
assignee: []
created_date: '2026-11-18 18:27'
updated_date: '2026-11-18 21:41'
labels:
  - code-review-rust
  - TIME
dependencies: []
parent_task_id: 'TASK-2838'
modified_files:
  - crates/backlog/src/cmd/create.rs
  - crates/backlog/src/cmd/edit.rs
  - crates/backlog/src/cmd/wave_create.rs
  - crates/backlog/src/cmd/wave.rs
priority: low
ordinal: 1000
dedup_key: 'TIME-6:crates/backlog/src/cmd/create.rs:create_task'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/create.rs:96` (`create_task`), `crates/backlog/src/cmd/edit.rs:58` (`run_edit`), `crates/backlog/src/cmd/wave_create.rs:73` (`run_wave_create`), `crates/backlog/src/cmd/wave.rs:424` (`migrate_with`)

**What**: `cleanup` and `about` read the clock once at the command edge (`cmd::wall_clock()`) and thread a `Timestamp` into an `_with` seam, as `cmd/mod.rs:68-79` documents. The writing commands instead call `UtcStamp::now()` deep inside the handler (and `run_edit` is itself called from `wave claim`/`wave park`), so `created_date`/`updated_date` cannot be pinned in tests: the create/edit golden tests can only assert shape, not the stamped value, and a minute rollover between two stamps in one command is untestable.

**Why it matters**: Inconsistent with the crate's own clock-injection convention; date-stamping behaviour (the field the cleanup age filter reads) has no deterministic test coverage.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 create_task, run_edit, run_wave_create and the wave migration take the stamp (or a Timestamp) as a parameter read once at the command edge
- [x] #2 At least one create and one edit test assert the exact created_date/updated_date from an injected fixed clock

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.01 (code-review-rust run, 2026-11-18).

Public run_* wrappers read UtcStamp::now() once at the edge and delegate to create_task(stamp) / edit_with / wave_create_with / migrate_with(stamp); wave claim/park read it at their own edge; create-coderabbit-tasks reads one stamp per review.

<!-- SECTION:NOTES:END -->
