---
id: TASK-0478
title: >-
  ERR-1: detect_package_manager has no bun arm despite lockfile probe
  recognizing bun.lock
status: To Do
assignee:
  - TASK-0532
created_date: '2026-04-28 05:48'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_manager.rs:11-19`

**What**: detect_package_manager maps "pnpm"/"yarn"/"npm" from the packageManager field but has no arm for "bun", even though the lockfile probe below recognizes bun.lockb / bun.lock.

**Why it matters**: A package.json declaring `"packageManager": "bun@1.1.0"` (the official way to pin Bun, supported by Corepack) returns None, falsely suggesting no package manager. The lockfile probe is only reached when the field is absent, so the explicit pin is silently dropped — an inconsistency between the two detection paths.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add  arm so the field-based path matches the lockfile-based path
- [ ] #2 Unit test asserts packageManager: "bun@1.1.0" resolves to Some("bun") even when no lockfile is present
<!-- AC:END -->
