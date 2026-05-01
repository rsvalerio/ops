---
id: TASK-0788
title: >-
  SEC-14: relative gitdir pointer joins file.parent() with attacker-controlled
  path before canonicalize, allowing symlink-bait worktree pointers
status: Triage
assignee: []
created_date: '2026-05-01 05:58'
labels:
  - code-review-rust
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:90`

**What**: read_gitdir_pointer returns Some(file.parent()?.join(target)) for relative pointers. The traversal cap (MAX_GITDIR_PARENT_TRAVERSAL = 2) limits .. segments, but Normal segments followed by ParentDir cancellations are unbounded — e.g. a/../../etc peaks at 1, passes the cap, and after file.parent() join lands one level above the worktree root. Subsequent canonicalize resolves through any symlinks the attacker may have planted.

**Why it matters**: SEC-14 path traversal containment. Hook installer writes into the resolved gitdir; even though is_accepted_git_dir validates filename shape, the canonicalize side effect runs first.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 After computing the joined path, canonicalize and verify the result starts_with(file.parent()?) (or a documented worktree-root anchor)
- [ ] #2 Add a regression test using a relative pointer that exercises Normal-then-ParentDir cancellation to ensure containment
<!-- AC:END -->
