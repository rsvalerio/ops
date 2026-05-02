---
id: TASK-0887
title: 'ERR-1: read_head_branch silently swallows non-NotFound IO errors via .ok()?'
status: Done
assignee: []
created_date: '2026-05-02 09:38'
updated_date: '2026-05-02 11:06'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/git/src/config.rs:160`

**What**: `read_head_branch` opens `<git_dir>/HEAD` with `std::fs::read_to_string(...).ok()?`, returning `None` for every IO error (NotFound, PermissionDenied, IsADirectory, EIO, etc.). Sister function `read_origin_url` in the same file (line 13) was deliberately upgraded to log non-NotFound errors at `tracing::warn!` per TASK-0548 / TASK-0517 / TASK-0711 to surface ACL or permission drift on `.git/config`; `read_head_branch` was left behind.

**Why it matters**: A `.git/HEAD` that becomes unreadable (mode 000 after a botched chown, EIO on a flaky disk, accidentally replaced with a directory) silently makes the ops `git_info` provider report `branch: None` as if HEAD were detached, when the real cause is an IO problem the operator could fix. Apply the established two-arm match: silent on NotFound, `tracing::warn!` on everything else, return None.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 non-NotFound IO errors emit tracing::warn! with path and error
- [ ] #2 test confirms unreadable HEAD (mode 0o000) returns None and emits a warn log
<!-- AC:END -->
