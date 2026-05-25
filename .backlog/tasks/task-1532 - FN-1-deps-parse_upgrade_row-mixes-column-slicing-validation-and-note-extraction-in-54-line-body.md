---
id: TASK-1532
title: >-
  FN-1: deps parse_upgrade_row mixes column slicing, validation, and note
  extraction in 54-line body
status: To Do
assignee:
  - TASK-1645
created_date: '2026-05-19 07:33'
updated_date: '2026-05-25 16:08'
labels:
  - code-review-rust
  - function-shape
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/parse/upgrade.rs:216-269`

**What**: The function does three distinct jobs: validate column count, slice the 5 fixed columns via a closure, then run a separate slicing pass for the optional `note`. Splitting into `slice_fixed_columns` + `slice_note_column` would isolate the optional/EOL semantics of the note from the fixed-column pattern.

**Why it matters**: The note path uses `line.len()` instead of `cols[5].end`, an asymmetry that's invisible inside a 54-line function. Splitting makes it explicit and testable.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract slice_fixed_columns(line, cols) -> Option<[&str; 5]> and slice_note(line, cols) -> Option<String>
- [ ] #2 parse_upgrade_row body shrinks to ~15 lines
- [ ] #3 parse_upgrade_table_with_notes and parse_upgrade_table_multi_word_note pass unchanged
<!-- AC:END -->
