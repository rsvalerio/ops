---
id: TASK-2864
title: 'SEC-25: wave create member updates and wave migrate rewrite task files outside the allocation lock'
status: To Do
assignee: []
created_date: '2026-11-18 21:21'
updated_date: '2026-11-19 07:57'
labels:
  - code-review-rust
  - SEC
dependencies: []
parent_task_id: 'TASK-2879'
modified_files:
  - crates/backlog/src/cmd/wave_create.rs
  - crates/backlog/src/cmd/wave.rs
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/wave_create.rs:102` (member update loop), `crates/backlog/src/cmd/wave.rs:477` (`wave migrate` write loop)

**What**: TASK-2682 put `task edit` (and so `wave claim` / `wave park`) under `Store::lock_allocation()` across its read-modify-write, and TASK-2670 put every `task create` under it. Two other writers still read a task, edit it in memory and `atomic_write` it back without the lock: `wave create` re-parents and re-statuses each member, and `wave migrate` rewrites marker-tagged tasks. A concurrent `task edit` on the same member (an agent appending notes, flipping status) can be silently overwritten by these rewrites, or overwrite them.

**Why it matters**: The same lost-update class TASK-2682 fixed for `task edit`, reached through the wave commands the code-review skills run while other agents edit member tasks.

**Origin**: discovered during TASK-2837 while fixing TASK-2682.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 wave create's member updates and wave migrate's rewrites hold Store::lock_allocation() across each task's read, modify and write (without deadlocking against create_task, which takes the same lock)
- [ ] #2 A concurrency test races a member rewrite against task edit --append-notes and asserts both changes survive
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
TASK-2838 (via TASK-2687) moved wave create's check, parent creation and member rewrites under the allocation lock; only the wave migrate half of this finding remains.
<!-- SECTION:NOTES:END -->
