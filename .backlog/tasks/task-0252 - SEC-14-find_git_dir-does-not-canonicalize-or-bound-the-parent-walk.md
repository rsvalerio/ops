---
id: TASK-0252
title: 'SEC-14: find_git_dir does not canonicalize or bound the parent walk'
status: Done
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 07:47'
labels:
  - rust-code-review
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:6`

**What**: is_dir() follows symlinks so a symlinked .git pointing outside the intended workspace is accepted; walk can ascend to filesystem root unbounded.

**Why it matters**: Provider may read .git/config from unexpected locations when cwd is attacker-controlled.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Canonicalize candidate and enforce a caller-supplied root ceiling
- [x] #2 Bound loop iterations; add test for symlinked .git outside workspace
<!-- AC:END -->
