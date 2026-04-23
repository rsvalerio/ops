---
id: TASK-0149
title: >-
  READ-5: find_git_dir treats any .git entry as a git dir — misses worktrees and
  submodules
status: Done
assignee: []
created_date: '2026-04-22 21:22'
updated_date: '2026-04-23 07:41'
labels:
  - rust-code-review
  - READ
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Files**:
- `extensions/hook-common/src/lib.rs:36`
- `extensions/git/src/config.rs:6`

**What**: `find_git_dir` walks upward checking `candidate.is_dir()`. In a git worktree or submodule, `.git` is a **file** (containing `gitdir: ...`), not a directory. Both implementations return `None` in that case and callers downstream (hook install, git_info provider) treat the working copy as "not a git repo".

**Why it matters**: Users working inside `git worktree`s silently cannot install hooks and see empty `git_info` results, with no diagnostic. Fix: when `.git` is a file, read it, parse `gitdir: <path>`, resolve relative to the worktree root, and return the resolved path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 find_git_dir handles the .git file (worktree/submodule) case
- [x] #2 Test added with a fixture .git file pointing to a sibling gitdir
<!-- AC:END -->
