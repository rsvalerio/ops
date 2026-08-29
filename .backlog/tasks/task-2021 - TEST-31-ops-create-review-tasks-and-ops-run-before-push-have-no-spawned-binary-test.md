---
id: TASK-2021
title: >-
  TEST-31: ops create-review-tasks and ops run-before-push have no
  spawned-binary test
status: To Do
assignee:
  - TASK-2046
created_date: '2026-08-28 19:37'
updated_date: '2026-08-29 11:35'
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
- [ ] #1 tests/integration.rs spawns 'ops create-review-tasks --dry-run' and asserts on its rendered plan output and exit code, without writing to the developer's backlog
- [ ] #2 tests/integration.rs spawns 'ops run-before-push' against a configured command and asserts the exit code is propagated for both a passing and a failing command
- [ ] #3 'ops run-before-push install' is covered for its non-TTY refusal: stderr names the command and the interactive-terminal requirement, and the exit code is non-zero
- [ ] #4 Every new test goes through the existing ops() helper so HOME/XDG_CONFIG_HOME isolation and OPS_* clearing are inherited
<!-- AC:END -->
