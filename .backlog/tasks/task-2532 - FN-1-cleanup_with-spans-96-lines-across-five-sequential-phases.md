---
id: TASK-2532
title: 'FN-1: cleanup_with spans ~96 lines across five sequential phases'
status: To Do
assignee: []
created_date: '2026-10-10 15:34'
updated_date: '2026-10-10 21:12'
labels:
  - code-review-rust
  - functions
dependencies: []
parent_task_id: 'TASK-2613'
modified_files:
  - crates/backlog/src/cmd/cleanup.rs
priority: medium
ordinal: 1000
dedup_key: 'FN-1:crates/backlog/src/cmd/cleanup.rs:cleanup_with'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/backlog/src/cmd/cleanup.rs:76` (cleanup_with, ~96 raw lines to line 172)

**What**: cleanup_with exceeds the 50-line FN-1 threshold. It walks five phases inline — terminal-status resolution, scan+filter, aged-filter with row printing, confirmation, then the preflight+move loops — mixing output rendering with the move policy it guards.

**Why it matters**: The destructive operation (move loop, lines 162-168) sits at the bottom of a function whose earlier half is print-formatting; extracting the aged-selection and the move-phase helpers separates the policy from the narration and brings each under the threshold.

<!-- scan confidence: raw-line measurement including comments -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 cleanup_with's phases (selection, preview, confirm, move) are named helpers; the function stays under ~50 lines
- [ ] #2 Preflight-then-move ordering and confirmation behaviour is unchanged (existing tests pass)
<!-- AC:END -->
