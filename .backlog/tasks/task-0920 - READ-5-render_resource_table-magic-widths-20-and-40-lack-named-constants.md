---
id: TASK-0920
title: 'READ-5: render_resource_table magic widths 20 and 40 lack named constants'
status: Triage
assignee: []
created_date: '2026-05-02 10:12'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-terraform/plan/src/render.rs:81`

**What**: `let capped = std::cmp::max(20, width.saturating_sub(40));` and `table.set_max_width(3, capped as u16);` hardcode the minimum (20) and reserved-for-other-columns (40) values. A reader cannot tell what 40 represents without inspecting the table column widths.

**Why it matters**: Mirrors TASK-0761 for the about extension table widths; future column additions silently drift the reservation budget.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Named consts (e.g. NAME_COL_MIN_WIDTH, OTHER_COLS_RESERVED) replace the literals
- [ ] #2 Doc comment explains how the reservation maps to the four-column header
<!-- AC:END -->
