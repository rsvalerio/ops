---
id: TASK-2106
title: >-
  DUP-2: create-review-tasks hand-rolls backlog frontmatter instead of reusing
  the ops-backlog TaskDoc renderer
status: To Do
assignee:
  - TASK-2242
created_date: '2026-09-08 06:53'
updated_date: '2026-09-08 10:57'
labels:
  - code-review-rust
  - duplication
dependencies: []
modified_files:
  - extensions/create-review-tasks/src/backlog.rs
  - crates/backlog/src/cmd/create.rs
priority: medium
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/backlog.rs:149` (`render_task_file`), against `crates/backlog/src/cmd/create.rs:71` (`Frontmatter` / `TaskDoc::render`)

**What**: The repository has two independent writers of the backlog.md task-file format. `crates/backlog/src/cmd/create.rs` builds an `ops_backlog::model::Frontmatter` and calls `TaskDoc::render()`; `extensions/create-review-tasks/src/backlog.rs::render_task_file` re-implements the same YAML frontmatter by hand with a sequence of `writeln!` calls, choosing its own field order and its own subset of keys (no `updated_date`, no `modified_files`, no body sections at all). Nothing but a pair of golden tests (`render_main_task_matches_cli_shape`, `render_subtask_matches_cli_shape`) keeps the two in agreement.

**Why it matters**: Any change to the canonical frontmatter shape — a new required key, a reordering, a change in how `yaml_scalar` quotes titles — has to be made in two places. Miss the second and `ops create-review-tasks` starts emitting task files the rest of the backlog tooling parses differently, with the golden tests still green because they were written against the old shape too. This is the classic "same format, two encoders" duplication: one encoder should own the format and the other should call it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 render_task_file builds its output through the shared ops_backlog frontmatter/task-document renderer rather than hand-written writeln! lines
- [ ] #2 The main-task and subtask golden tests still pass byte-for-byte against the shapes the backlog CLI writes
- [ ] #3 Any create-review-tasks-specific fields (label sets, ordinals, parent_task_id) are expressed as inputs to the shared renderer, not as a second implementation of it
<!-- AC:END -->
