---
id: TASK-2092
title: >-
  READ-6: task-file writes are truncate-in-place, inconsistent with the crate's
  own crash-safe move
status: To Do
assignee:
  - TASK-2235
created_date: '2026-09-07 22:59'
updated_date: '2026-09-08 10:54'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - crates/backlog/src/cmd/edit.rs
  - crates/backlog/src/cmd/wave.rs
priority: medium
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/edit.rs:135`, `crates/backlog/src/cmd/edit.rs:177`, `crates/backlog/src/cmd/wave.rs:255`

**What**: Task rewrites use `std::fs::write`, which truncates the destination in place before writing — a crash or power loss mid-write leaves a truncated/corrupt task file. `rename_to_new_slug` (edit.rs:164) additionally writes the new name then removes the old one non-atomically, so a crash between the two leaves two files carrying the same task id. In `migrate_with` (wave.rs:253) the per-file write loop has no all-or-nothing story: a write failure at file 3 of 10 produces exactly the half-applied migration whose own preflight comment says must not exist ("a half-applied migration would leave membership split across two conventions with no record of which tasks were done").

**Why it matters**: The crate already models the right posture twice — `move_to_completed` claims the destination atomically via hard-link + EEXIST with a documented crash analysis, and `run_create` uses `File::create_new` — so the edit/migrate paths solving the same class of problem with the naive shape is a consistency gap, not a style preference. Write-temp-then-rename gives each single-file rewrite atomicity; reusing the hard-link claim (or a staging preflight) extends it to the rename and multi-file migrate paths.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Task-file rewrites are atomic: write to a temp file in the same directory then rename over the destination
- [ ] #2 The title-rename path cannot leave two live files with one task id after a crash (or documents the same recovery story move_to_completed does)
- [ ] #3 wave migrate either applies a file completely or not at all, or reports exactly which files were written before the failure
<!-- AC:END -->
