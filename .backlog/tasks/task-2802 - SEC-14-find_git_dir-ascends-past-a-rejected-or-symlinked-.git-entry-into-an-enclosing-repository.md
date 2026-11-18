---
id: TASK-2802
title: 'SEC-14: find_git_dir ascends past a rejected or symlinked .git entry into an enclosing repository'
status: Done
assignee: []
created_date: '2026-11-18 18:46'
updated_date: '2026-11-18 22:21'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2861'
modified_files:
  - extensions/hook-common/src/git.rs
priority: medium
ordinal: 1000
dedup_key: 'SEC-14:extensions/hook-common/src/git.rs:find_git_dir'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:62` (`find_git_dir`), `extensions/hook-common/src/git.rs:82` (`probe_git_entry`)

**What**: When the walk finds a `.git` entry at a level but refuses it, `probe_git_entry` returns `None` and `find_git_dir` pops to the parent directory and keeps searching. That happens for:
- a symlinked `.git` (git.rs:85 — "skipped silently"),
- a `.git` pointer file that is unreadable (git.rs:128-144), malformed or has two `gitdir:` lines (git.rs:155-162),
- a pointer whose target fails the traversal cap, the anchor check, or the absolute-target rules (git.rs:168-183, 320-338).

Git itself stops at the first `.git` entry it finds and reports an error; it never treats an enclosing repository as the owner of a nested checkout. Here the walk instead returns the *enclosing* repository's gitdir, and `install_hook` then writes an executable hook into that unrelated repository (e.g. a superproject when the submodule pointer is refused, or a `$HOME` dotfiles repo when the project's `.git` is a symlink).

**Why it matters**: The refusal paths exist precisely because the result feeds a write primitive that produces a script git executes on every commit. Falling through turns each refusal into a redirect to a different repository — the hook is installed where the operator did not ask, and the repository they did ask about stays unprotected while `install` reports success. The existing test `find_git_dir_skips_symlinked_dot_git` only passes because no ancestor of the tempdir is a repository.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A .git entry that exists at a level but is refused (symlink, unreadable/malformed pointer, containment rejection) terminates the walk with None (or a typed error) instead of ascending to the parent
- [x] #2 A test places a real .git in an outer directory and a refused .git (symlink, and a rejected pointer file) in a nested directory, and asserts find_git_dir on the nested directory does not return the outer gitdir
- [x] #3 The doc comment on find_git_dir states the stop-at-first-.git-entry contract

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.27 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
