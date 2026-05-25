---
id: TASK-1153
title: >-
  READ-5: layout_cards_in_grid public entry silently picks 120-col fallback on
  non-TTY callers
status: Done
assignee:
  - TASK-1271
created_date: '2026-05-08 07:43'
updated_date: '2026-05-09 07:46'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:165-167`

**What**: `layout_cards_in_grid` calls `get_terminal_width()` which falls back to 120 columns when stdout is not a TTY and COLUMNS is unset. The doc warns to use `layout_cards_in_grid_with_width` for buffer-writing callers — but both functions are pub with no compile-time differentiation. A new caller picks the silent 120-col fallback that ERR-1 / TASK-0784 explicitly fixed downstream.

**Why it matters**: The fix only landed on one consumer; documentation alone enforces the contract. The next caller can grab the wrong entry point and inherit the 120-col bug.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Make layout_cards_in_grid pub(crate) and force every external consumer through layout_cards_in_grid_with_width
- [ ] #2 Or rename to layout_cards_in_grid_for_stdout so the call site signals intent
<!-- AC:END -->
