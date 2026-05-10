---
id: TASK-0548
title: >-
  ERR-1: try_read_manifest silently swallows non-NotFound IO errors in workspace
  glob expansion
status: Done
assignee:
  - TASK-0638
created_date: '2026-04-29 05:01'
updated_date: '2026-04-29 10:34'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:78`

**What**: try_read_manifest uses std::fs::read_to_string(...).ok() to coerce every IO failure into "no manifest". A glob entry whose package.json/pyproject.toml exists but is unreadable (permissions, partial write, EIO) silently drops the unit from the workspace listing. The surrounding resolve_member_globs was specifically updated for ERR-1 (TASK-0517) to log read_dir failures, but this leaf retains the silent shape.

**Why it matters**: Inconsistent with the read_dir arm two layers up; produces the same "No project units found" silent failure mode TASK-0517 set out to fix — just one level deeper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 try_read_manifest distinguishes ErrorKind::NotFound (return None) from other errors (warn and return None, or propagate)
- [ ] #2 A unit test exercises the unreadable-manifest branch and pins that behaviour
<!-- AC:END -->
