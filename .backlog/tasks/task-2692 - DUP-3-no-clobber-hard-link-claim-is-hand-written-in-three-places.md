---
id: TASK-2692
title: 'DUP-3: no-clobber hard-link claim is hand-written in three places'
status: Done
assignee: []
created_date: '2026-11-18 18:28'
updated_date: '2026-11-18 21:26'
labels:
  - code-review-rust
  - DUP
dependencies: []
parent_task_id: 'TASK-2837'
modified_files:
  - crates/backlog/src/cmd/mod.rs
  - crates/backlog/src/cmd/edit.rs
  - crates/backlog/src/cmd/cleanup.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-3:crates/backlog/src/cmd/cleanup.rs:move_to_completed'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/mod.rs:219-234` (`atomic_write_noclobber`), `crates/backlog/src/cmd/edit.rs:183-196` (`rename_to_new_slug`), `crates/backlog/src/cmd/cleanup.rs:253-269` (`move_to_completed`)

**What**: The same sequence — `std::fs::hard_link(from, to)`, map `AlreadyExists` to "`{to} already exists; refusing to overwrite it with {from}`", wrap other errors with "linking ...", then `remove_file` the source — is written out three times with slightly different messages and cleanup behaviour (mod.rs ignores the source-removal error, cleanup.rs and edit.rs propagate it). Each copy carries its own long doc comment re-explaining why `rename` is unsafe.

**Why it matters**: The no-clobber move is the crate's core data-safety primitive; three copies mean a fix (e.g. fsyncing the directory, Windows fallback) lands in one and not the others.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A single helper (e.g. link_noclobber / move_noclobber in cmd/mod.rs) owns the hard-link claim and its error messages
- [x] #2 rename_to_new_slug, move_to_completed and atomic_write_noclobber call it; existing tests pass

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.01 (code-review-rust run, 2026-11-18).

Fixed in TASK-2837: cmd::link_noclobber owns the hard-link claim and the DestinationTaken error; atomic_write_noclobber, rename_to_new_slug and move_to_completed call it.

<!-- SECTION:NOTES:END -->
