---
id: TASK-0225
title: 'FN-9: layout_cards_in_grid reads COLUMNS env var as implicit input'
status: To Do
assignee: []
created_date: '2026-04-23 06:33'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - function-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:144`

**What**: `get_terminal_width()` inside the function reads process env rather than taking width as a parameter.

**Why it matters**: Hidden dependency on global state; complicates testing and reasoning.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Accept term_width: usize parameter
- [ ] #2 Update call sites to pass explicit width
<!-- AC:END -->
