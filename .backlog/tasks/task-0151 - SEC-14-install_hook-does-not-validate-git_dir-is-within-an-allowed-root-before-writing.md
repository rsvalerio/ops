---
id: TASK-0151
title: >-
  SEC-14: install_hook does not validate git_dir is within an allowed root
  before writing
status: Done
assignee: []
created_date: '2026-04-22 21:22'
updated_date: '2026-04-23 07:42'
labels:
  - rust-code-review
  - SEC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:52`

**What**: `install_hook(config, git_dir, ...)` appends `"hooks"` and `config.hook_filename` to the caller-supplied `git_dir` and calls `fs::write`. There is no canonicalization or bound-checking against a cwd root. If a caller accidentally passes a symlinked or attacker-writable path as `git_dir`, the function will happily write an executable `0o755` file wherever that path resolves.

**Why it matters**: Defense-in-depth. Today only CLI entry points call this with the discovered `.git` dir, but as a library crate its contract should require the directory be canonicalized and, ideally, validated to end in `.git` or `.git/worktrees/<name>`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 install_hook documents and enforces that git_dir is a canonicalized .git directory
- [x] #2 Reject paths that are not .git/ or a worktree gitdir
<!-- AC:END -->
