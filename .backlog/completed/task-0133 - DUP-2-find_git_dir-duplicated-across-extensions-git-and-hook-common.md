---
id: TASK-0133
title: 'DUP-2: find_git_dir duplicated across extensions/git and hook-common'
status: Done
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-04-23 07:34'
labels:
  - label1
  - label2
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:6-17` and `extensions/hook-common/src/lib.rs:36-47`

**What**: The same 12-line ancestor-walking `find_git_dir` function is defined verbatim in two crates.

**Why it matters**: DRY: fixes (e.g., handling bare repos, submodules, worktrees) must be applied to both copies, and the two will drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Canonicalize find_git_dir in one crate (ops-hook-common or ops-git) and re-export if needed
- [x] #2 The other crate depends on the canonical implementation rather than redefining it
<!-- AC:END -->
