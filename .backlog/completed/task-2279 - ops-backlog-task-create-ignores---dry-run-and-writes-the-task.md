---
id: TASK-2279
title: 'ops backlog task create ignores --dry-run and writes the task'
status: Done
assignee: []
created_date: '2026-09-26 15:25'
updated_date: '2026-09-26 18:40'
labels:
  - bug
  - cli
  - dry-run
  - backlog
dependencies: []
parent_task_id: 'TASK-2290'
modified_files:
  - crates/cli/src/backlog_cmd.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops backlog task create "<title>" ... --dry-run` creates the task file anyway. The `--dry-run` listed in `ops backlog task create --help` is the global flag ("Preview commands without executing"), and it never reaches the backlog code path.

**Where**: `crates/cli/src/backlog_cmd.rs` — only `CreateReviewTasks { dry_run }` and `WaveMigrate { dry_run }` carry a dry-run; `task create` (and presumably the other mutating backlog actions: `task edit`, archive, etc.) never receive `cli.dry_run`.

**Why it matters**: the help text advertises a preview, so a caller relying on it (a skill eval, a script checking a command's arguments) silently writes `.backlog/tasks/` files and allocates task ids. Confirmed 2026-09-26 in a scratch repo while evaluating the `rust-make-build-fast` skill (rsvalerio/ai).

**Repro**: in a repo with a backlog, `ops backlog task create "x" --dry-run --plain` → prints `Created TASK-000N` and the file exists.

**Fix sketch**: either honour the flag for every mutating backlog action (print the task that would be written, allocate no id), or reject `--dry-run` on backlog actions with an error — never accept it and ignore it.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `ops backlog task create ... --dry-run` writes no file and allocates no id, or fails with an explicit error
- [x] #2 The same holds for every other mutating backlog action
- [x] #3 A test pins the chosen behaviour

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Chosen behaviour: under the global --dry-run, `backlog init`, `task create` and `task edit` (the mutating actions with no preview mode) fail with an explicit error before anything is read or written; cleanup / create-review-tasks / wave migrate keep their own preview; read-only actions run unchanged. Classification is an exhaustive match (backlog_cmd::unpreviewable_mutation) so a new mutating action must be classified to compile.
<!-- SECTION:NOTES:END -->
