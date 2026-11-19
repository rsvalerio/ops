---
id: TASK-2786
title: 'SEC-25: create-review-tasks allocates ids outside the backlog allocation lock and can duplicate a concurrent task create''s id'
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
  - extensions/create-review-tasks/src/backlog.rs
priority: high
ordinal: 1000
dedup_key: 'SEC-25:extensions/create-review-tasks/src/lib.rs:commit_task_set'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/lib.rs:456-474` (`commit_task_set`), `extensions/create-review-tasks/src/lib.rs:481-538` (`write_task_set`), `extensions/create-review-tasks/src/backlog.rs:80-99` (`next_ids`), `extensions/create-review-tasks/src/backlog.rs:146-170` (`conflicting_claim`)

**What**: The review-request writer allocates its main-task number by scanning the tree (`next_ids`), creating the main file with `create_new`, and then re-reading the tree (`conflicting_claim`) to detect a concurrent claim of the same number under a different filename. It never takes `ops_backlog::store::Store::lock_allocation()`, the exclusive advisory lock that `ops backlog task create --unless-exists` holds around its own scan+create (`crates/backlog/src/cmd/create.rs` `run_create`). The re-check protects only against writers that also re-check. A keyed `task create` does not, so this interleaving yields two tasks with one id:

1. `task create` (holding the lock) scans, max = N, picks N+1.
2. `create-review-tasks` scans, max = N, picks N+1, `create_new`s `task-(N+1) - review-request-<date>-<n>.md` (different slug, so it succeeds).
3. `conflicting_claim` re-reads the tree; the CLI file is not written yet, so the claim looks uncontested and the set is kept.
4. `task create` now `create_new`s `task-(N+1) - <its slug>.md`, which also succeeds, and it does not re-check.

The lock exists precisely to serialise allocation across processes, and this second allocator bypasses it. Related: TASK-2670, the same defect inside `ops backlog task create` itself.

**Why it matters**: Duplicate ids break `find` (first match wins), parent/subtask links, wave membership and `ops backlog commit`. Review agents running `ops backlog task create` while someone runs `ops backlog create-review-tasks` is the documented concurrent workflow.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 commit_task_set holds Store::lock_allocation() across id allocation and every task-file create of an attempt, so it serialises with keyed ops backlog task create
- [x] #2 A test interleaves a lock-holding ops-backlog create with a create-review-tasks run and asserts distinct main-task numbers
- [x] #3 The doc comments on commit_task_set and conflicting_claim describe the lock as the guarantee, and keep the re-check only if it still covers writers that do not take the lock

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.25 (code-review-rust run, 2026-11-18).

Fixed in TASK-2837: commit_task_set holds Store::lock_allocation across all attempts; conflicting_claim kept for writers outside the lock (backlog.md CLI, hand-copied files). Test: a_run_waits_for_a_lock_holding_writer_and_takes_a_distinct_number (fails without the lock).

<!-- SECTION:NOTES:END -->
