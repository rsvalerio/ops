---
id: TASK-2640
title: 'Small .ops.toml ergonomics: built-in foundation command and a default category'
status: To Do
assignee: []
created_date: '2026-10-26 22:12'
updated_date: '2026-10-31 23:55'
labels:
  - feature
  - config
dependencies: []
parent_task_id: 'TASK-2645'
modified_files: []
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Two repetitions seen in dbsec's .ops.toml:
- Every Rust repo on the foundation defines [commands.foundation] program = "ops", args = ["init", "--rust", "--check"] so qa can name it. A built-in foundation command (Rust stack) would remove that, and the ops pin of the outer process would be used instead of whatever ops is on PATH.
- category = "Code Quality" is repeated on ~15 commands. A default, e.g. a [commands] level or per-section default category, or inheriting category from the composite a command is listed in, would remove the repetition.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ops foundation works with no [commands.foundation] entry on a Rust stack
- [ ] #2 A decision recorded on default categories
<!-- AC:END -->
