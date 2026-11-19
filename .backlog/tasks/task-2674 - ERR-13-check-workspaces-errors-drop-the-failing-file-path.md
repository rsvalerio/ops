---
id: TASK-2674
title: 'ERR-13: check-workspaces errors drop the failing file path'
status: Done
assignee: []
created_date: '2026-11-18 18:26'
updated_date: '2026-11-18 22:07'
labels:
  - code-review-rust
  - ERR
dependencies: []
parent_task_id: 'TASK-2843'
modified_files:
  - crates/cli/src/workspace_check_cmd.rs
priority: low
ordinal: 1000
dedup_key: 'ERR-13:crates/cli/src/workspace_check_cmd.rs:run_workspace_check'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/workspace_check_cmd.rs:295`, `:296`, `:297`, `:309` (`manifests_in::walk`), `:407` (`check_lock`), `:441` (`check_lints`), `:502` (`run_workspace_check`), `:112` (`parse_lock`)

**What**: Several `?` propagations in `ops check-workspaces` carry no path:
- `manifests_in::walk`: `std::fs::read_dir(dir)?`, `entry?`, `entry.file_type()?`, `std::fs::read_to_string(entry.path())?` — the only context is the outer "walking <root>", so an unreadable nested `Cargo.toml` or directory is reported as "walking /repo: Permission denied".
- `check_lock`: `std::fs::read_to_string(manifest_path)?` has no context (the `.parse()` after it does).
- `check_lints` (`snapshot(..)?.parse()?`) and `run_workspace_check` (`snapshot(&root.join("Cargo.toml"))?.parse()?`) propagate a bare TOML parse error with no file name.
- `parse_lock` says "parsing a Cargo.lock" without saying which one (root or `<entry>/Cargo.lock`).

**Why it matters**: The command checks several lockfiles and manifests per run; a bare io/TOML error in a CI log does not say which file is broken.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Every fs and TOML-parse error in workspace_check_cmd names the file it concerns
- [x] #2 A test asserts the path appears in the error for an unparsable nested Cargo.toml

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.02 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
