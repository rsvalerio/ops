---
id: TASK-0361
title: 'SEC-25: TOCTOU between is_file HEAD check and hook write in install path'
status: Done
assignee:
  - TASK-0416
created_date: '2026-04-26 09:36'
updated_date: '2026-04-27 08:27'
labels:
  - code-review-rust
  - security
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/hook-common/src/paths.rs:60`

**What**: looks_like_git_dir does a path.join("HEAD").is_file() check, but subsequent file operations in install.rs happen later under independent std::fs calls. An attacker who can swap the directory between check and create_new can pass the .git-shape gate.

**Why it matters**: Allows redirected write to a non-git directory. Real-world exploit requires write access to a parent path; severity medium because of that precondition.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Document the TOCTOU window explicitly with a SAFETY comment OR switch to handle-based open (openat-equivalent) for the HEAD check
- [x] #2 A regression test simulates a swap between check and use and asserts either rejection or no out-of-tree write
<!-- AC:END -->
