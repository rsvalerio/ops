---
id: TASK-2659
title: 'Accept ops backlog task <id> as shorthand for ops backlog task view <id>'
status: Done
assignee: []
created_date: '2026-11-13 18:36'
updated_date: '2026-11-13 19:15'
labels:
  - feature
  - backlog
  - cli
dependencies: []
modified_files:
  - crates/cli/src/args.rs
  - crates/cli/src/backlog_cmd.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops backlog task 1765 --plain` fails with `error: unrecognized subcommand '1765'`; only `ops backlog task view 1765 --plain` works. The backlog.md CLI accepts `backlog task <id>` as a view, and agents reach for that form.

**Why it matters**: seen in a dbsec review run (2026-11-12): the agent called `ops backlog task <id> --plain`, got the error, then ran `which backlog`, found the backlog.md binary and finished the job with `backlog task edit ...`, outside ops. The shorthand removes the failure that pushed it off ops.

**Shape**: when the first positional after `task` is a task id (`TASK-N`, `N`, `N.M`, case-insensitive) rather than a known subcommand, dispatch to `view` with the remaining flags (`--plain`, `--json`). Subcommand names keep priority, so no id can shadow `create`/`edit`/`list`/`view`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 ops backlog task 1765 --plain prints the same output as ops backlog task view 1765 --plain
- [x] #2 TASK-1765, 1765 and 1764.02 forms are all accepted; --json is forwarded
- [x] #3 An argument that is neither a subcommand nor an id still fails with the unrecognized-subcommand error

<!-- AC:END -->
