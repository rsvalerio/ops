---
id: TASK-0022
title: Redundant gather_* tests duplicated across 3 hook modules
status: Done
assignee: []
created_date: '2026-04-10 22:00:00'
updated_date: '2026-04-11 09:55'
labels:
  - rust-test-quality
  - TQ
  - TEST-12
  - TEST-5
  - medium
  - crate-cli
dependencies: []
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: `crates/cli/src/run_before_commit_cmd.rs:117-194`, `crates/cli/src/run_before_push_cmd.rs:117-194`, `crates/cli/src/run_before_commit_cmd.rs:189-256`
**Anchor**: `fn gather_excludes_*`, `fn gather_merges_config_and_stack`, `fn gather_config_takes_priority_over_stack`
**Impact**: Nine tests across three hook modules (`run_before_commit_cmd`, `run_before_push_cmd`, `run_before_commit_cmd`) exercise the **same** function — `hook_shared::gather_available_commands` — with identical scenarios. The only difference between the three copies is the `exclude_name` argument (`"run-before-commit"`, `"run-before-push"`, `"run-before-commit"`). This is redundant per TEST-12: identical logic, same code paths, copy-paste with trivial differences. Meanwhile, `hook_shared.rs` itself has zero direct tests (TEST-5 gap).

**Notes**:
- The three redundant test groups are:
  - `gather_excludes_<hook>` — tests that the hook's own command is excluded from the list (3 copies)
  - `gather_merges_config_and_stack` — tests config + stack merge (3 identical copies)
  - `gather_config_takes_priority_over_stack` — tests config priority (3 identical copies)
- All nine tests call `hook_shared::gather_available_commands` which is a single shared function
- Fix: move the gather_* tests into a `#[cfg(test)]` module in `hook_shared.rs`, parameterize by `exclude_name` to cover the exclusion behavior once, and delete the duplicates from the three hook modules
- The install_* tests (3 per module) are NOT redundant — they test different hook extension libraries (`ops_run_before_commit`, `ops_run_before_push`, run-before-commit) and verify different hook file paths, so they should remain
<!-- SECTION:DESCRIPTION:END -->
