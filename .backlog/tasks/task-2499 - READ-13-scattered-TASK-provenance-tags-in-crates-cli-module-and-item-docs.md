---
id: TASK-2499
title: 'READ-13: scattered TASK provenance tags in crates/cli module and item docs'
status: To Do
assignee: []
created_date: '2026-10-10 15:31'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - readability
dependencies: []
parent_task_id: 'TASK-2611'
modified_files:
  - crates/cli/src/backlog_cmd.rs
  - crates/cli/src/dry_run.rs
  - crates/cli/src/extension_cmd.rs
  - crates/cli/src/import_makefile_cmd.rs
  - crates/cli/src/init_rust_cmd.rs
  - crates/cli/src/lint_actions_cmd.rs
  - crates/cli/src/msrv_cmd.rs
  - crates/cli/src/new_command_cmd.rs
  - crates/cli/src/subcommands.rs
priority: low
ordinal: 1000
dedup_key: 'READ-13:crates/cli/src:scattered module docs'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/backlog_cmd.rs:267`, `crates/cli/src/dry_run.rs:1`, `crates/cli/src/extension_cmd.rs:265`, `crates/cli/src/import_makefile_cmd.rs:198`, `crates/cli/src/init_rust_cmd.rs:1`, `crates/cli/src/lint_actions_cmd.rs:1`, `crates/cli/src/msrv_cmd.rs:1`, `crates/cli/src/new_command_cmd.rs:57,74`, `crates/cli/src/subcommands.rs:331`

**What**: One or two doc-comment lines per file carry TASK/RULE provenance tags: module openers ("//! TASK-2301: how each builtin subcommand treats the global --dry-run", "//! Handler for ops init --rust ... (TASK-2330)", "//! ... supply-chain policy (TASK-2328)", "//! ... rust-version (TASK-2327)") and item docs ("DUP-1 / TASK-1449: writer-injected entry point", "READ-5 (TASK-1355): writer-injection seam", "ERR-10 (TASK-1316): returns anyhow::Result", "TASK-2279: the backlog actions ... are refused", "TASK-2278: preview a hook action ...", "PATTERN-1 / TASK-1653" in is_recipe_line).

**Why it matters**: Self-report tags naming the guideline a change followed are READ-13's named shape; they are meaningless to the next reader and go stale while looking authoritative. Domain uses ("Task id (e.g. TASK-0042)" for backlog ids in args.rs) are NOT violations and must stay.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each listed doc line states the end state with the TASK/RULE tag removed
- [ ] #2 Backlog-domain TASK-id examples in args.rs are left untouched
<!-- AC:END -->
