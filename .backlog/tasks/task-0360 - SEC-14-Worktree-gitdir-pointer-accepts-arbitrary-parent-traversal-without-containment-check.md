---
id: TASK-0360
title: >-
  SEC-14: Worktree gitdir pointer accepts arbitrary parent traversal without
  containment check
status: Done
assignee:
  - TASK-0419
created_date: '2026-04-26 09:36'
updated_date: '2026-04-27 10:52'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/git.rs:57`

**What**: read_gitdir_pointer resolves the gitdir: value relative to the pointer file parent without verifying the resolved path stays within an expected root. A worktree pointer like "gitdir: ../../../../../etc" would be returned and later written into by the hook installer.

**Why it matters**: A malicious repository checked out by a developer could redirect hook installation to attacker-controlled directories. Downstream looks_like_git_dir HEAD check mitigates blind writes, but defense-in-depth is missing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 read_gitdir_pointer rejects pointers whose resolved path traverses above the workdir root
- [x] #2 Test added with gitdir: ../../../etc/passwd asserting find_git_dir returns None even when HEAD is planted in the resolved target
<!-- AC:END -->
