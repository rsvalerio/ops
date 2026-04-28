---
id: TASK-0517
title: 'ERR-1: resolve_member_globs swallows read_dir errors silently'
status: To Do
assignee:
  - TASK-0534
created_date: '2026-04-28 06:52'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:32`

**What**: `if let Ok(entries) = std::fs::read_dir(&parent)` ignores any error (EACCES, ENOENT-on-glob-prefix). The user's broken workspace silently shows zero members.

**Why it matters**: User-facing about/units page reports "No project units found" when the real cause is a permissions or missing-directory issue. No log.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Log read_dir errors at debug or warn level
- [ ] #2 Test that a permission-denied on a glob prefix produces a log line
<!-- AC:END -->
