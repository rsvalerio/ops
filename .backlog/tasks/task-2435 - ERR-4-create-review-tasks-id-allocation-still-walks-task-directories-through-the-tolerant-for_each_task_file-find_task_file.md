---
id: TASK-2435
title: 'ERR-4: create-review-tasks id allocation still walks task directories through the tolerant for_each_task_file / find_task_file'
status: Triage
assignee: []
created_date: '2026-10-04 15:56'
labels:
  - code-review-rust
  - ERR
dependencies: []
modified_files:
  - extensions/create-review-tasks/src/backlog.rs
  - extensions/create-review-tasks/src/lib.rs
  - crates/backlog/src/store.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/backlog.rs:82` (`next_ids`), `extensions/create-review-tasks/src/backlog.rs:142` (`conflicting_claim`), `crates/backlog/src/store.rs` (`for_each_task_file`, `find_task_file`)

**What**: `ops_backlog::store::Store::next_task_number` now fails, naming the directory, when a task directory or one of its entries cannot be read. The two public free walkers `for_each_task_file` and `find_task_file` keep a tolerant contract (an unreadable directory or entry is skipped like an absent one, no error channel), and `create-review-tasks` allocates its main-task number, its review-request sequence and its conflict re-check through them.

**Why it matters**: an unreadable `completed/` or `archive/` directory makes `next_ids` allocate from a partial listing, so a review request can reuse a task number or sequence that lives only in the unreadable directory, with no diagnostic. The `ops backlog task create` path no longer has this gap; the extension does.

**Origin**: discovered during TASK-2423 while fixing TASK-2354. The walkers' signatures were left unchanged there because their only callers are in `extensions/create-review-tasks`, outside that wave's file scope.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 next_ids and conflicting_claim surface an unreadable task directory or entry as an error naming the directory instead of allocating from a partial listing
- [ ] #2 for_each_task_file and find_task_file either return a Result or are removed in favour of a fallible walker shared with Store::next_task_number
<!-- AC:END -->
