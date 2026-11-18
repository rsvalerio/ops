---
id: TASK-2807
title: 'TEST-12: git_timeout_from_env tests re-run ops_hook_common''s parser cases through a one-line wrapper'
status: Done
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 22:30'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2861'
modified_files:
  - extensions/run-before-commit/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-12:extensions/run-before-commit/src/lib.rs:tests::git_timeout_from_env'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:810-836` (`git_timeout_from_env_valid_value`, `git_timeout_from_env_zero_falls_back`, `git_timeout_from_env_unparseable_falls_back`, `git_timeout_from_env_unset_returns_none`)

**What**: `git_timeout_from_env` (`:130`) is a one-line delegation to `ops_hook_common::git_state::git_timeout_from_env(TIMEOUT_ENV_VAR, MAX_GIT_TIMEOUT_SECS)`. These four tests repeat, case for case, the parser tests already in `extensions/hook-common/src/git_state.rs:286-312` (same inputs `"10"`, `"0"`, `"10s"`, unset; same expected results), differing only in the env var name.

**Why it matters**: They verify the shared parser a second time rather than anything this crate contributes, and each takes the `serial` lock on process env. What the wrapper adds — that `TIMEOUT_ENV_VAR` is the variable read and `MAX_GIT_TIMEOUT_SECS` is the ceiling — is already pinned by `has_staged_files_applies_the_env_timeout` (`:747`) and `git_timeout_from_env_clamps_to_ceiling` (`:841`) / `clamp_log_emission`, which should stay.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The four redundant parser-case tests are removed (or reduced to one test that pins TIMEOUT_ENV_VAR wiring)
- [x] #2 The clamp-to-ceiling, clamp-warn and env-timeout-applied tests remain

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.28 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
