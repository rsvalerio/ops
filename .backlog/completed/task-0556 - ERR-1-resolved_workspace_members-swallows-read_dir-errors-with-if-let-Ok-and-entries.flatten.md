---
id: TASK-0556
title: >-
  ERR-1: resolved_workspace_members swallows read_dir errors with if-let-Ok and
  entries.flatten()
status: Done
assignee:
  - TASK-0638
created_date: '2026-04-29 05:02'
updated_date: '2026-04-29 10:35'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/query.rs:93-102`

**What**: When expanding crates/* in resolved_workspace_members, std::fs::read_dir errors are silently dropped via if let Ok(entries) = ... and per-entry errors via entries.flatten(). TASK-0517 was filed against the sibling resolve_member_globs, not this about-rust copy.

**Why it matters**: A misconfigured crates/ directory (permission denied, EIO) yields zero members and silently produces empty about/units/coverage views with no signal — the failure mode TASK-0376 / TASK-0517 close elsewhere.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Log read_dir failure at warn matching the unsupported-glob warn already there
- [ ] #2 Replace entries.flatten() with explicit per-entry error logging
<!-- AC:END -->
