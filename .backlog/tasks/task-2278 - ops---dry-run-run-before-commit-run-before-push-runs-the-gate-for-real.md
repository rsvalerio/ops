---
id: TASK-2278
title: 'ops --dry-run run-before-commit / run-before-push runs the gate for real'
status: Triage
assignee: []
created_date: '2026-09-26 15:25'
labels:
  - bug
  - cli
  - dry-run
dependencies: []
modified_files:
  - crates/cli/src/main.rs
priority: high
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops --dry-run run-before-push` and `ops --dry-run run-before-commit` ignore the global `--dry-run` flag and execute the whole gate — deps, nextest, test-doc, sec, clippy-default, then `verify` including the rewriters `fmt`, `trailing-whitespace` and `end-of-file-fixer`, and `cargo doc`.

**Where**: `crates/cli/src/main.rs` dispatches `CoreSubcommand::RunBeforeCommit { .. }` / `RunBeforePush { .. }` to `run_before_commit(early_config, action, changed_only)` / `run_before_push(early_config, action)` without `cli.dry_run`, unlike `sec` (`sec_cmd::run_sec(&cwd, cli.dry_run, ..)`) and composite commands, which honour it.

**Why it matters**: `--dry-run` is documented as "Preview commands without executing"; a preview that rewrites files is the opposite of what the flag promises. Found 2026-09-26 while evaluating the `rust-make-build-fast` skill (rsvalerio/ai), whose survey step used `ops --dry-run <gate>` to read gate plans; it started rewriters in ops before being killed. The tree came out unchanged only by luck.

**Repro**: `cd ops && ops --dry-run run-before-push` → the gate runs.

**Fix sketch**: thread `cli.dry_run` into both hook paths and resolve/print the plan (as composites do) instead of running it; for `run-before-commit`, also preview the `install`/hook action variants rather than writing hooks.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `ops --dry-run run-before-commit` and `ops --dry-run run-before-push` print the resolved plan and execute no step
- [ ] #2 A test pins that no step runs under --dry-run for both hook subcommands
<!-- AC:END -->
