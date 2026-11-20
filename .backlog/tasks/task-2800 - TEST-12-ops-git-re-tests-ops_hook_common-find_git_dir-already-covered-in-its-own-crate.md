---
id: TASK-2800
title: 'TEST-12: ops-git re-tests ops_hook_common::find_git_dir already covered in its own crate'
status: Done
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 22:30'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2861'
modified_files:
  - extensions/git/src/config.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-12:extensions/git/src/config.rs:tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:566`, `:575`, `:586` (`tests::find_git_dir_in_current`, `find_git_dir_in_parent`, `find_git_dir_missing`)

**What**: `config.rs` only re-exports `find_git_dir` from `ops_hook_common` (`pub(crate) use ops_hook_common::find_git_dir;`, line 7), yet its test module carries three tests of that foreign function. They duplicate `find_git_dir_in_current`, `find_git_dir_in_parent` and `find_git_dir_not_found` in `extensions/hook-common/src/git.rs:441-461`, which also covers worktree pointers, symlinked `.git` and traversal rejection. The ops-git copies exercise none of this crate's logic.

**Why it matters**: Redundant tests that exercise another crate add build/run time and maintenance load without protecting ops-git behaviour, and they imply ownership of `find_git_dir` semantics in the wrong crate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The three find_git_dir_* tests are removed from extensions/git/src/config.rs, or replaced by a test that pins ops-git's own use of find_git_dir through GitInfo::collect
- [x] #2 ops_hook_common keeps the canonical find_git_dir tests

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.26 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
