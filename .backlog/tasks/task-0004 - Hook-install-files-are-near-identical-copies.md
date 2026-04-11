---
id: TASK-0004
title: Hook install files are near-identical copies
status: Done
assignee: []
created_date: '2026-04-10 14:30:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-code-duplication
  - CD
  - DUP-1
  - DUP-2
  - medium
  - crate-cli
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/cli/src/run_before_commit_cmd.rs:1-194`, `crates/cli/src/run_before_push_cmd.rs:1-194`, `crates/cli/src/run_before_commit_cmd.rs:10-56`
**Anchor**: `fn run_before_commit_install`, `fn run_before_push_install`, `fn run_before_commit_install`
**Impact**: Three hook install commands share identical orchestration logic (~150 lines of production code duplicated across `before_commit_cmd.rs` and `before_push_cmd.rs`, with the same pattern in `pre_commit_cmd.rs`). Each file follows the same flow: TTY check → find git dir → load config → resolve stack → gather commands → MultiSelect prompt → install hook → ensure config command. The only differences are module import names (`ops_run_before_commit` vs `ops_run_before_push` vs `ops_run_before_commit`), command name strings, and hook file names. The test suites (~140 lines each) are also structurally identical, varying only in the same string literals.

**Notes**:
`hook_shared.rs` already extracted `gather_available_commands` and `command_description`, but the full install orchestration was not parameterized. A fix would extract a generic `run_hook_install` function in `hook_shared.rs` that accepts the hook-specific details (command name, display name, install/configure closures) as parameters. Each file would then be a thin wrapper calling the shared function with its specifics.

The near-identity between `run_before_commit_cmd.rs` and `run_before_push_cmd.rs` is especially stark — the files are line-for-line copies with only 5–6 string substitutions. `run_before_commit_cmd.rs` differs slightly more (it has a `#[cfg(test)]` helper with a different shape) but the production `run_*_install()` function body is the same pattern.

DUP-1: 5+ identical lines across files. DUP-2: 3+ functions with similar structure differing only in types/literals.
<!-- SECTION:DESCRIPTION:END -->
