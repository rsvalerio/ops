---
id: TASK-2777
title: 'SEC-14: repo_relative_file strips the worktree root as a string prefix, not by path component'
status: Done
assignee: []
created_date: '2026-11-18 18:44'
updated_date: '2026-11-18 23:05'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2860'
modified_files:
  - extensions/create-coderabbit-tasks/src/finding.rs
priority: low
ordinal: 1000
dedup_key: 'SEC-14:extensions/create-coderabbit-tasks/src/finding.rs:repo_relative_file'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/create-coderabbit-tasks/src/finding.rs:331-342` (`repo_relative_file`)

**What**: `repo_relative_file` removes the worktree root with `str::strip_prefix(worktree_root)` and then trims leading `/`. That is a character-prefix match, not a path-component match: with root `/tmp/wt`, the CodeRabbit `fileName` `/tmp/wt2/src/lib.rs` (a sibling directory, outside the worktree) becomes `2/src/lib.rs` and is accepted as a repo path. The escape check that follows only looks for a leading `/` and literal `..` segments, so it cannot catch this.

**Why it matters**: The function exists to refuse paths outside the repository before they go into a task's `modified_files` and its dedup key. SEC-14 asks for component-wise validation against the allowed root. Exploitation is unlikely (the root is a fresh temp directory), but the guard as written is weaker than its doc comment says.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The worktree root is stripped with Path::strip_prefix (component-wise), and an absolute path not under the root is refused
- [x] #2 The escape check rejects Component::ParentDir, RootDir and Prefix rather than only the literal string '..'
- [x] #3 A unit test asserts that /tmp/wt2/x with root /tmp/wt is refused

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.24 (code-review-rust run, 2026-11-18).

Fixed in wave TASK-2860: repo_relative_file takes &Path, strips the root with Path::strip_prefix, refuses absolute paths outside it, and rejects ParentDir/RootDir/Prefix components. Test: a_sibling_directory_sharing_the_root_as_a_string_prefix_is_refused.

<!-- SECTION:NOTES:END -->
