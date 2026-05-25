---
id: TASK-1070
title: >-
  ERR-1: about workspace strip_prefix failure silently drops a successfully-read
  manifest from the resolved set
status: Done
assignee: []
created_date: '2026-05-07 21:18'
updated_date: '2026-05-08 06:36'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:64`

**What**: When `root` contains a symlink component but `entry.path()` resolves to the target form (or vice versa), `strip_prefix` returns `Err` and the `if let Ok(rel) = ...` branch silently discards the unit. Operators see "no project units found" with no breadcrumb.

**Why it matters**: A successful manifest read is dropped without any log. Symlinked workspaces (common on macOS dev setups via /Users/.../private/var) hit this routinely and the failure mode is invisible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Emit tracing::debug! (or warn) on strip_prefix failure with both paths so the drop is observable
- [x] #2 Add a regression test pinning behaviour when root is a symlinked path
- [x] #3 Consider canonicalising both sides before the strip
<!-- AC:END -->
