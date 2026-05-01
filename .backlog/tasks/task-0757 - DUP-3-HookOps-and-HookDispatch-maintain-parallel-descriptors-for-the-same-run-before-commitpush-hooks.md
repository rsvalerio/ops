---
id: TASK-0757
title: >-
  DUP-3: HookOps and HookDispatch maintain parallel descriptors for the same
  run-before-{commit,push} hooks
status: Triage
assignee: []
created_date: '2026-05-01 05:53'
labels:
  - code-review-rust
  - duplication
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/pre_hook_cmd.rs:8-20` and `crates/cli/src/subcommands.rs:106-130`

**What**: HookOps holds hook_name, find_git_dir, install_hook, ensure_config_command. HookDispatch holds name, skip_env_var, should_skip, preflight, install — with install being a function that itself calls run_before_{commit,push}_install which dispatches to HookOps. Two parallel constant tables describe the same hooks.

**Why it matters**: Adding a third hook (e.g. run-before-merge) requires editing two parallel tables in two modules. The split obscures that they describe a single hook contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Two descriptor structs collapse to one HookSpec (or HookOps extended with skip/preflight fields), defined once and used by both subcommands::run_hook_dispatch and hook_shared::run_hook_install*
- [ ] #2 pre_hook_cmd only adds the per-hook constant; no parallel struct definition
- [ ] #3 Tests in pre_hook_cmd::tests and subcommands::tests continue to pass with the unified type
<!-- AC:END -->
