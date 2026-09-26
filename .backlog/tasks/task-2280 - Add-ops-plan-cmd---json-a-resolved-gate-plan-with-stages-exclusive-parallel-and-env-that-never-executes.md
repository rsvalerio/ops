---
id: TASK-2280
title: 'Add ops plan <cmd> --json: a resolved gate plan with stages, exclusive, parallel and env that never executes'
status: To Do
assignee: []
created_date: '2026-09-26 17:47'
updated_date: '2026-09-26 18:27'
labels:
  - feature
  - cli
  - skills-integration
dependencies: []
parent_task_id: 'TASK-2290'
modified_files:
  - crates/cli/src/run_cmd/plan.rs
  - crates/cli/src/run_cmd/dry_run.rs
  - crates/cli/src/args.rs
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: a read-only `ops plan <cmd> [--json]` that resolves a command exactly as running it would, and prints the plan without running any step:

- every step with its program, args, env and cwd
- `parallel` / `fail_fast` of each composite
- the **stages**: exclusive steps as barriers, and each run of consecutive non-exclusive steps as one concurrent stage
- where each step came from: stack default, `.ops.toml`, `[extend.*]`, or `clone = "..."` (with the inherited or overridden `exclusive`)

**Why**: `ops --dry-run <cmd>` prints steps, but shows no `exclusive`, `parallel` or `env`. So the `rust-make-build-fast` skill has to rebuild the stages by hand from `.ops.toml`, plus hard-coded knowledge of the stack's built-in exclusive steps (`fmt`, `trailing-whitespace`, `end-of-file-fixer`) and of `clone` inheritance. Worse, `--dry-run` is not guaranteed read-only: see TASK-2278, where `ops --dry-run run-before-push` runs the whole gate. A separate `plan` subcommand can promise, and test, that it never executes anything, whatever the command name.

**Used by**: `rust-make-build-fast` (GATE-1/2/3, TEST-1 checks), and anyone auditing gate layout.
Source: skills-vs-ops audit of rsvalerio/ai dev-skills, 2026-09-26 (https://claude.ai/artifact/4yaKVkPdfe93hrqhFW5u1z).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `ops plan <cmd> --json` emits steps, stages, parallel/fail_fast, exclusive, env and origin for any composite or exec command
- [ ] #2 A test pins that `ops plan` executes no step for any command, including run-before-commit and run-before-push
- [ ] #3 The JSON carries a schemaVersion, like the backlog JSON output
<!-- AC:END -->
