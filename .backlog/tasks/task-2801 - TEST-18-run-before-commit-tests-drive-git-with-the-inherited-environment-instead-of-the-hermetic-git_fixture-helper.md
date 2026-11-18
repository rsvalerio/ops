---
id: TASK-2801
title: 'TEST-18: run-before-commit tests drive git with the inherited environment instead of the hermetic git_fixture helper'
status: To Do
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2861'
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: high
ordinal: 1000
dedup_key: 'TEST-18:extensions/run-before-commit/src/lib.rs:tests::init_repo'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:397` (`init_repo`), `:483` (`commit_file`), plus inline `git add`/`git rm` calls at `:472`, `:506`, `:531`, `:730`

**What**: The test module builds its fixture repositories with bare `std::process::Command::new("git")`, inheriting the developer's/CI's full environment: global and system git config (`commit.gpgsign`, `core.hooksPath`, `init.templateDir`, `core.excludesFile`) and any `GIT_DIR` / `GIT_WORK_TREE` / `GIT_INDEX_FILE` in scope. The workspace already ships a hermetic helper for exactly this, `ops_core::test_utils::git_fixture` (`crates/core/src/test_utils.rs:707`), which points `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` at `/dev/null` and scrubs the three location env vars; this crate already depends on `ops-core` with `test-support` but does not use it.

**Why it matters**: This crate's own hook runs `ops run-before-commit` from inside `git commit`, where git exports `GIT_INDEX_FILE` (and may export `GIT_DIR`). If the configured command chain runs `cargo test`, `git add a.txt` / `git rm doomed.txt` in these tests resolve against the outer repo's index rather than the tempdir, either failing spuriously or mutating the developer's staged commit. Separately, a global `commit.gpgsign = true` makes `commit_file` prompt or fail, and a global `core.hooksPath` runs the developer's hooks inside the fixture commit — machine-dependent flakiness (see flakiness-patterns).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 init_repo, commit_file and the inline git add/rm fixture calls go through ops_core::test_utils::git_fixture (or an equivalent hermetic wrapper)
- [ ] #2 The run-before-commit test suite passes when run with GIT_INDEX_FILE/GIT_DIR pointing at another repository and with a global config setting commit.gpgsign=true
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.28 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
