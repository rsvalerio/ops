---
id: TASK-2500
title: 'API-14: crate-local pub fns across crates/cli lack doc summaries'
status: Done
assignee: []
created_date: '2026-10-10 15:31'
updated_date: '2026-10-10 21:30'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2611'
modified_files:
  - crates/cli/src/about_cmd.rs
  - crates/cli/src/args.rs
  - crates/cli/src/extension_cmd.rs
  - crates/cli/src/hook_shared.rs
  - crates/cli/src/import_makefile_cmd.rs
  - crates/cli/src/init_cmd.rs
  - crates/cli/src/init_rust_cmd.rs
  - crates/cli/src/new_command_cmd.rs
  - crates/cli/src/pre_hook_cmd.rs
  - crates/cli/src/row.rs
  - crates/cli/src/run_cmd/dry_run.rs
  - crates/cli/src/run_cmd/plan.rs
  - crates/cli/src/subcommands.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:crates/cli:crate-local pub fns'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/about_cmd.rs:12`, `crates/cli/src/args.rs:1008`, `crates/cli/src/extension_cmd.rs:21,253`, `crates/cli/src/hook_shared.rs:225`, `crates/cli/src/import_makefile_cmd.rs:32`, `crates/cli/src/init_cmd.rs:9`, `crates/cli/src/init_rust_cmd.rs:17`, `crates/cli/src/new_command_cmd.rs:8`, `crates/cli/src/pre_hook_cmd.rs:34,43`, `crates/cli/src/row.rs:39`, `crates/cli/src/run_cmd/dry_run.rs:35,105`, `crates/cli/src/run_cmd/plan.rs:74`, `crates/cli/src/subcommands.rs:25,131,141,151,379,449,458,492,498`

<!-- scan confidence: candidates to inspect -->

**What**: 24 `pub fn` items in production code have no `///` summary on the item (verified on samples; full candidate list above). The crate documents the large majority of its functions well, including `# Errors` sections (see lock_cmd.rs), so these are the residue, concentrated in subcommands.rs thin wrappers and handler entry points (run_about, run_before_commit, run_theme, run_extension, run_init, run_import_makefile, ...).

**Why it matters**: API-14 wants every public item to carry a doc summary; `pub` here is crate-internal plumbing of a binary crate (many are one-line delegations to documented implementation crates), so a one-line summary naming the subcommand and where the real docs live is enough. Manual triage: wrappers whose callee doc fully covers them may be resolved by pointing or by demoting to `pub(crate)` where no cross-module use exists.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every listed pub fn carries a /// summary (one line naming the subcommand and the documented callee is acceptable)
- [x] #2 Items demoted to pub(crate) instead of documented also satisfy the finding

<!-- AC:END -->
