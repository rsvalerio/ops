---
id: TASK-2111
title: >-
  READ-11: create-review-tasks hardcodes the TASK prefix and 4-digit padding
  that BacklogConfig makes configurable
status: To Do
assignee:
  - TASK-2242
created_date: '2026-09-08 06:53'
updated_date: '2026-09-08 10:57'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions/create-review-tasks/src/backlog.rs
priority: medium
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-review-tasks/src/backlog.rs:188` (`main_task_id`), `:193` (`main_task_file_name`), `:199` (`subtask_id`), `:205` (`subtask_file_name`)

**What**: All four id/filename helpers bake in the literal id width `4` and, in two of them, the literal prefix `TASK`:

- `main_task_id` -> `format_task_id("TASK", number, 4)`
- `main_task_file_name` -> `main_task_file_name(number, 4, title)`
- `subtask_id` -> `format!("TASK-{number:04}.{index:02}")` — bypasses `format_task_id` entirely
- `subtask_file_name` -> `format!("task-{number:04}.{index:02} - {}.md", ...)` — bypasses `main_task_file_name`'s formatting

Meanwhile `ops_backlog::config::BacklogConfig` exposes `task_prefix` and `zero_padded_ids` (parsed from the repo's backlog config, default 4), and `crates/backlog/src/cmd/create.rs:67-68` honours both.

**Why it matters**: Two things. (1) In a repository whose backlog config sets a different `zero_padded_ids` or `task_prefix`, `ops create-review-tasks` writes ids and filenames that disagree with every other task the backlog CLI creates in the same tree — the review request is then a second, incompatible id namespace. (2) The width is a magic literal repeated four times with two call sites re-implementing the formatting inline, so a change to the shared helpers silently leaves the subtask paths behind. There is no named constant and no comment explaining why 4 is fixed here.

<!-- If the fixed width is deliberate (byte-compatibility with a specific CLI), that rationale belongs in a named const, not in four repeated literals. -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The id width and task prefix come from one place — either the repo's BacklogConfig or a single documented const in this module — not from four repeated literals
- [ ] #2 subtask_id and subtask_file_name derive their formatting from the shared ops_backlog helpers instead of inlining {number:04}
- [ ] #3 If the fixed width is intentional, a doc comment states why create-review-tasks does not follow BacklogConfig::zero_padded_ids
<!-- AC:END -->
