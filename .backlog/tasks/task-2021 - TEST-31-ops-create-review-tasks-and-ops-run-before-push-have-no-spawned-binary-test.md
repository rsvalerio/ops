---
id: TASK-2021
title: >-
  TEST-31: ops create-review-tasks and ops run-before-push have no
  spawned-binary test
status: Done
assignee:
  - TASK-2046
created_date: '2026-08-28 19:37'
updated_date: '2026-08-29 12:44'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - crates/cli/tests/integration.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/tests/integration.rs`

**What**: TASK-1737 added spawned-process coverage for the README command
table — the fixer/checker pre-commit exit-code contract, `ops sec --dry-run`,
the all-skipped `ops sec` fail-closed path, `ops extension list` / `show`, and
the non-TTY refusal of `ops new-command` / `ops import-makefile`. Two commands
named in that finding's description were outside its acceptance criteria and
remain uncovered as processes:

- `ops create-review-tasks` (and its `--dry-run` form via the global flag)
- `ops run-before-push` (and `ops run-before-push install`)

Neither is run as a binary by any test, so their argument spellings, stdout /
stderr routing, and exit codes are unverified at the CLI boundary.

**Why it matters**: TEST-31 — a CLI's real interface is the binary. Both
commands are gate-shaped: `run-before-push` is invoked by a git pre-push hook
and its exit code decides whether a push proceeds, so a regression that
collapsed a failure to exit 0 would silently disarm the hook, which is the same
failure class TASK-1737 closed for the fixers.

**Origin**: discovered during TASK-1982 while fixing TASK-1737.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tests/integration.rs spawns 'ops create-review-tasks --dry-run' and asserts on its rendered plan output and exit code, without writing to the developer's backlog
- [x] #2 tests/integration.rs spawns 'ops run-before-push' against a configured command and asserts the exit code is propagated for both a passing and a failing command
- [x] #3 'ops run-before-push install' is covered for its non-TTY refusal: stderr names the command and the interactive-terminal requirement, and the exit code is non-zero
- [x] #4 Every new test goes through the existing ops() helper so HOME/XDG_CONFIG_HOME isolation and OPS_* clearing are inherited
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Covered in crates/cli/tests/integration.rs (wave TASK-2046):

- `cli_create_review_tasks_dry_run_reports_the_plan_without_writing` — spawns
  `ops create-review-tasks --dry-run` in a tempdir carrying its own
  `.backlog/tasks`, asserts the `would create` plan lines and the
  `list subtasks:` footer, and asserts the backlog tree is still empty
  afterwards. The developer's backlog is never in scope: the fixture is the
  process cwd.
- `cli_create_review_tasks_writes_the_task_set` — the write form the finding's
  description also named, asserting the `created` report and the two task
  files it must agree with.
- `cli_create_review_tasks_without_a_backlog_tree_fails` — the refusal names
  `.backlog/tasks` rather than only exiting non-zero.
- `cli_run_before_push_propagates_the_configured_command_exit_code` — a
  configured `true` exits 0 and a configured `false` exits non-zero, which is
  the property the git pre-push hook reads.
- `cli_run_before_push_without_a_configured_command_fails_closed` — the
  unconfigured hook must not exit 0.
- `cli_run_before_push_install_without_a_terminal_refuses` — stderr names
  `run-before-push install` and the interactive-terminal requirement; exit is
  non-zero.

Every new test goes through the existing `ops()` helper; the run-before-push
ones wrap it in `ops_hook()`, which additionally clears
`SKIP_OPS_RUN_BEFORE_PUSH` — that variable does not match the `OPS_*` prefix
`ops()` strips, so an ambient bypass would otherwise have made both exit-code
assertions vacuous.
<!-- SECTION:NOTES:END -->
