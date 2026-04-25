---
id: TASK-0310
title: >-
  SEC-14: is_accepted_git_dir relies on filename heuristics instead of
  validating the directory is a real git dir
status: Done
assignee:
  - TASK-0325
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 12:51'
labels:
  - rust-code-review
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions/hook-common/src/install.rs:100-122

**What**: The acceptance check uses file_name == .git or a .git/worktrees/* ancestor match after canonicalization. An attacker-controlled path canonicalizing to such a name is accepted without verifying it's actually a git repo.

**Why it matters**: Weakens the hook-install boundary; low impact (local user context) but the check should validate substance, not filename.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Validate by opening <git_dir>/HEAD or config as a sanity check
- [ ] #2 Add test for a bogus /tmp/.git that isn't a repo — installer must reject
<!-- AC:END -->
