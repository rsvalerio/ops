---
id: TASK-2131
title: >-
  PERF-3: each allocation attempt walks the whole backlog tree twice, and
  conflicting_claim keeps walking after it has found its answer
status: To Do
assignee:
  - TASK-2244
created_date: '2026-09-08 06:55'
updated_date: '2026-09-08 10:58'
labels:
  - code-review-rust
  - performance
dependencies: []
modified_files:
  - extensions/create-review-tasks/src/backlog.rs
  - crates/backlog/src/store.rs
priority: low
ordinal: 47000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/backlog.rs:121` (`conflicting_claim`), `:61` (`next_ids`); driven from `extensions/create-review-tasks/src/lib.rs:290` (`commit_task_set`)

**What**: One commit attempt performs two full walks of every task directory (`tasks`, `completed`, and both archives): `next_ids` for the allocation, then `conflicting_claim` for the post-reservation re-check. `commit_task_set` retries this up to `MAX_ALLOCATION_ATTEMPTS` (32) times, so a contended run can walk the tree 64 times.

`conflicting_claim` also cannot stop early: `for_each_task_file` takes a `FnMut` with no way to break, so once a conflict is found the closure sets `conflict` and then simply returns on every remaining entry — the directory walk still enumerates the entire tree.

**Why it matters**: This repository's `.backlog/tasks` already holds a couple of thousand files, and it only grows. The DUP-1 note on `next_ids` explicitly counts halving the per-attempt directory I/O as a win, so the second walk in the same attempt — and the walk that continues past a decided answer — is the same cost the module already decided was worth avoiding. The re-check itself is load-bearing and must stay; what is avoidable is scanning the whole tree to answer a question already answered.

<!-- scan confidence: the cost is structural, not measured; the fix is an early-exit form of for_each_task_file (a callback returning ControlFlow, or a `_while` variant) plus, if practical, reusing one listing per attempt. -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 conflicting_claim stops walking as soon as it has found a conflicting file, via an early-exit traversal rather than a flag checked per entry
- [ ] #2 The existing conflicting_claim and next_ids tests still pass unchanged, including the own-file exemption and the completed/archive cases
- [ ] #3 Any new early-exit traversal helper added to ops_backlog::store is documented and covered by a test
<!-- AC:END -->
