---
id: TASK-2682
title: 'SEC-25: task edit is an unlocked read-modify-write, so concurrent edits of one task lose updates'
status: To Do
assignee: []
created_date: '2026-11-18 18:27'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2837'
modified_files:
  - crates/backlog/src/cmd/edit.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-25:crates/backlog/src/cmd/edit.rs:run_edit'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/edit.rs:50-72` (`run_edit`)

**What**: `run_edit` reads and parses the task (`store.find`), applies the edits in memory, then `atomic_write`s the whole re-rendered document. Nothing serializes this against another writer: two concurrent `task edit` calls on the same task (e.g. two agents each `--append-notes`, or `wave claim`'s status edit racing a subtask agent's note) both read version N and the second rename wins, silently discarding the first edit. `atomic_write` only guarantees the file is never half-written, not that updates compose. The same pattern is reached via `run_wave_claim` / `run_wave_park` (both call `run_edit`).

**Why it matters**: The crate is explicitly used by concurrently running review/wave agents; lost status flips or notes go unnoticed and break the workflow's bookkeeping (e.g. a tracking task's note listing created ids disappears).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Edits to one task are serialized (e.g. the store's allocation lock or a per-task lock held across read, modify and write) or detected via a compare-before-rename check that retries/fails loudly
- [ ] #2 A concurrency test runs N parallel --append-notes edits on one task and asserts all N notes are present
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.01 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
