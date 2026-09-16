---
id: TASK-2263
title: 'Run the sec builtin exclusively in parallel plans'
status: To Do
assignee: []
created_date: '2026-09-16 17:01'
updated_date: '2026-09-16 17:12'
labels:
  - bug
  - runner
  - sec
dependencies: []
parent_task_id: 'TASK-2265'
modified_files:
  - crates/runner/src/command/builtins.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
`crates/runner/src/command/builtins.rs` registers `sec` with `read_only(...)`, which sets `exclusive = false` so it can overlap other steps in a parallel plan. It never writes the worktree, but Trivy reads the entire tree, including build directories that concurrent steps (cargo build, nextest with trybuild, and so on) are creating and deleting. Trivy aborts the whole scan when a file vanishes mid-walk (`fs scan error ... no such file or directory`), so running `sec` next to a build or test step is a race, not a safe overlap.

Keeping `sec` exclusive makes it run alone, in list order, in any parallel plan. This matters even once named commands get their own plans (TASK-2262), because users can list `sec` in their own parallel composites.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `sec` is registered exclusive (drop the `read_only` wrapper for it); `check-json` and `check-yaml` stay non-exclusive
- [ ] #2 The comment on `read_only` says why `sec` is excluded: it reads the whole tree, build outputs included
- [ ] #3 A test pins `sec` as exclusive in `builtin_commands()`
<!-- AC:END -->
