---
id: TASK-0140
title: 'UNSAFE-1: terse SAFETY comments around termios MaybeUninit in terminal.rs'
status: To Do
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - unsafe
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/terminal.rs:39, 46, 52, 77`

**What**: SAFETY comments exist on each unsafe block but are one-liners. E.g., "tcgetattr succeeded, so termios is fully initialized" doesn't spell out the MaybeUninit assume_init contract or what would break it.

**Why it matters**: UNSAFE-1 requires each unsafe block to justify the required invariants in enough detail that a future maintainer can re-verify them without re-deriving from the libc docs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Expand each SAFETY comment to state (a) the precondition, (b) why it holds here, (c) what would violate it
- [ ] #2 Confirm assume_init is only called on success paths of the corresponding libc call
<!-- AC:END -->
