---
id: TASK-0273
title: 'READ-5: builtin_category return type Option is unused — every arm returns Some'
status: Done
assignee: []
created_date: '2026-04-23 06:36'
updated_date: '2026-04-23 15:22'
labels:
  - rust-code-review
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/help.rs:25`

**What**: No arm returns None; wrapper is dead; new unmapped name silently slots into "Commands".

**Why it matters**: Return type misleads readers and masks missing-category gaps.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Return &'static str directly
- [ ] #2 Or reserve None with a warn log
<!-- AC:END -->
