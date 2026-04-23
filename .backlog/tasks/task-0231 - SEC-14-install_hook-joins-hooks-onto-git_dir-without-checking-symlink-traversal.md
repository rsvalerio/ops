---
id: TASK-0231
title: >-
  SEC-14: install_hook joins 'hooks' onto git_dir without checking symlink
  traversal
status: Done
assignee: []
created_date: '2026-04-23 06:34'
updated_date: '2026-04-23 07:43'
labels:
  - rust-code-review
  - security
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/lib.rs:57`

**What**: `git_dir.join("hooks")` not canonicalized — a symlinked hooks dir can redirect writes outside the repo.

**Why it matters**: Complements prior allowed-root check: even after that, hooks/ itself could symlink out.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Canonicalize hooks_dir and verify it is within git_dir
- [x] #2 Reject symlinked hooks directory
<!-- AC:END -->
