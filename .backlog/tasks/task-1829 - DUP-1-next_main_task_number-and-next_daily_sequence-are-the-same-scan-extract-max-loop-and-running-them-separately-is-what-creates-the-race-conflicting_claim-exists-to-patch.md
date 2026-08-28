---
id: TASK-1829
title: >-
  DUP-1: next_main_task_number and next_daily_sequence are the same
  scan-extract-max loop, and running them separately is what creates the race
  conflicting_claim exists to patch
status: To Do
assignee:
  - TASK-2005
created_date: '2026-08-27 11:34'
updated_date: '2026-08-28 14:16'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions/create-review-tasks/src/backlog.rs
  - extensions/create-review-tasks/src/lib.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/backlog.rs:50-75` (callers: `extensions/create-review-tasks/src/lib.rs:186-187`, `227-243`)

**What**: The two allocators are structurally identical — same `join(".backlog")`, same `let mut max = 0u32`, same `for_each_task_file` walk, same `max = max.max(n)`, same `max.saturating_add(1)`. They differ only in the extractor closure (`leading_task_number` vs `review_request_sequence(.., &prefix)`):

```rust
pub fn next_main_task_number(workspace_root: &Path) -> u32 {
    let backlog_root = workspace_root.join(".backlog");
    let mut max = 0u32;
    for_each_task_file(&backlog_root, |_dir, file_name| {
        if let Some(n) = leading_task_number(file_name) { max = max.max(n); }
    });
    max.saturating_add(1)
}
```

`next_daily_sequence` is the same seven lines with one substitution. Both are called back-to-back from `plan_task_set`, so every allocation attempt walks all four `TASK_DIRS` **twice**, and `conflicting_claim` then walks them a **third** time — inside a loop bounded by `MAX_ALLOCATION_ATTEMPTS = 32`, i.e. up to 96 full walks of `.backlog` for one command.

The duplication is not just cosmetic: the two walks being separate calls is precisely the defect the design has to compensate for. `commit_task_set`'s own doc comment says so — "ids come from directory scans... the two scans are not even atomic with each other" (lib.rs:217-219) — and `conflicting_claim`'s doc repeats it — "Allocation reads the tree twice — once for the number, once for the daily sequence — and those scans are not atomic with each other" (backlog.rs:102-104). A single pass that accumulates both maxima from one `for_each_task_file` walk makes the two values consistent with each other by construction and removes that interleaving from the set of races the post-reservation re-check has to cover.

**Why it matters**: Two near-identical loops drift independently — and they already have (see the sibling READ-6 finding: one anchors its filename parse, the other substring-matches). Collapsing them to one walk returning a `(next_number, next_sequence)` pair cuts the per-attempt directory I/O by a third, removes the only structural reason the two ids can be read from different states of the tree, and gives a single place to fix filename parsing. `for_each_task_file` already takes a `FnMut`, so the merge is a closure that runs both extractors — no new abstraction is needed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 next_main_task_number and next_daily_sequence no longer each perform their own for_each_task_file walk; one pass over TASK_DIRS yields both the next main-task number and the next daily sequence
- [ ] #2 plan_task_set performs at most one backlog-tree walk per allocation attempt (down from two)
- [ ] #3 the two returned ids are derived from the same directory listing, and the doc comments on commit_task_set and conflicting_claim are updated to drop the now-false 'reads the tree twice / not atomic with each other' rationale, or to state precisely which race remains
- [ ] #4 existing backlog.rs allocation tests (next_number_* and daily_sequence_* families) pass unchanged
<!-- AC:END -->
