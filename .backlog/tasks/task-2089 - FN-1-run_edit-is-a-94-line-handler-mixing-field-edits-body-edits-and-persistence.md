---
id: TASK-2089
title: 'FN-1: run_edit is a 94-line handler mixing field edits, body edits, and persistence'
status: Done
assignee: []
created_date: '2026-09-07 22:58'
updated_date: '2026-09-09 18:26'
labels:
  - code-review-rust
  - structure
dependencies: []
parent_task_id: 'TASK-2246'
modified_files:
  - crates/backlog/src/cmd/edit.rs
priority: medium
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/edit.rs:48`

**What**: `run_edit` spans 94 lines and some 20 sequential `if let` / `for` blocks operating at three different abstraction levels: frontmatter field mutation (status, assignees, labels, parent, deps, priority, title), body section mutation (description, ac, dod, notes — via re-parse-and-rewrite calls), and persistence (stamp, render, rename-vs-write branch). Every block is individually simple; the length comes from the levels being interleaved in one function.

**Why it matters**: FN-1's threshold (50 lines) exists for exactly this shape: the function is the natural unit of review and testing, and a reviewer must hold all three levels to verify one edit kind. Decomposing — e.g. `apply_frontmatter_edits(&mut fm, opts)` and `apply_body_edits(&mut doc, opts) -> Result<()>` around a thin persist step — keeps each helper at one level and makes the mapping from EditOptions field to mutation locally checkable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 run_edit orchestrates named helpers (frontmatter edits, body edits, persistence) and is itself under 50 lines
- [x] #2 Each extracted helper operates at a single abstraction level
- [x] #3 Existing edit tests pass unchanged (behaviour is identical)

<!-- AC:END -->
