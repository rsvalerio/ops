---
id: TASK-0188
title: >-
  READ-1: ProgressDisplay struct doc block claims 'if grows beyond ~500 lines of
  non-test code' while the file is 675 lines
status: To Do
assignee: []
created_date: '2026-04-22 21:25'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:62-77` (ProgressDisplay doc block) and the surrounding module (676 lines).

**What**: The struct-level doc on `ProgressDisplay` says "If this struct grows beyond ~50 methods or 500 lines of non-test code, consider extracting: ProgressState, EventRouter". The module itself is already 676 lines including the struct. A reader following the stated heuristic would check the line count and be misled — the comment gives the impression the refactor trigger has not been hit when in fact it has.

**Why it matters**: READ-1 (clarity / don't let comments lie). Either update the doc block to reflect current reality ("...if this grows beyond 800 lines, extract ProgressState..."), remove the speculative heuristic, or actually perform the extraction now. TASK-0110 (ARCH-1 on display.rs) is marked Done for a previous split; this is the follow-up drift.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Reconcile the 'when to split' heuristic in ProgressDisplay\'s doc block with the current line count
<!-- AC:END -->
