---
id: TASK-2289
title: 'Add ops backlog wave claim/park: claim a wave with branch, worktree and status in one step'
status: To Do
assignee: []
created_date: '2026-09-26 17:47'
updated_date: '2026-09-26 18:27'
labels:
  - feature
  - backlog
  - waves
  - git
  - skills-integration
dependencies: []
parent_task_id: 'TASK-2291'
modified_files:
  - crates/backlog/src/cmd/wave.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**What**: `ops backlog wave claim <wave-id>` creates the wave branch and worktree (`git worktree add -b`, whose failure on an existing branch is the claim's exclusivity) and flips the wave parent to In Progress, as one operation. `ops backlog wave park <wave-id>` records a failed merge: the branch and worktree are kept for resumption, and the status and a note are updated.

**Why**: in `code-review-run-wave` the claim is a `git worktree add -b` followed by a separate status edit. The protocol has to spell out the order (claim first, then flip the status) and what to do when the branch already exists. Parking is likewise several manual steps.

Lowest priority of this set: the current shell works. This mainly removes steps an agent can get out of order.

**Used by**: code-review-run-wave.
Source: skills-vs-ops audit of rsvalerio/ai dev-skills, 2026-09-26 (https://claude.ai/artifact/4yaKVkPdfe93hrqhFW5u1z).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 `wave claim` fails without side effects when the wave branch already exists
- [ ] #2 A successful claim leaves the branch, the worktree and In Progress status all in place
- [ ] #3 `wave park` keeps the branch and worktree and records why the wave was parked
<!-- AC:END -->
