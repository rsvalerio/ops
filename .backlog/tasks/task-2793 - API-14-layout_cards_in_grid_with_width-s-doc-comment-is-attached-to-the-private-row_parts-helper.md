---
id: TASK-2793
title: 'API-14: layout_cards_in_grid_with_width''s doc comment is attached to the private row_parts helper'
status: To Do
assignee: []
created_date: '2026-11-18 18:45'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - API
dependencies: []
parent_task_id: 'TASK-2858'
modified_files:
  - extensions/about/src/cards.rs
priority: low
ordinal: 1000
dedup_key: 'API-14:extensions/about/src/cards.rs:layout_cards_in_grid_with_width'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:175-203` (`layout_cards_in_grid_with_width`, `row_parts`)

**What**: The doc block "Lay out a slice of pre-rendered cards into a grid sized for `term_width`" with its render contract (lines 175-191) runs straight into the `row_parts` doc (lines 192-198) with no item between them, so rustdoc attaches the whole block to the private `fn row_parts`. The public `pub fn layout_cards_in_grid_with_width` (line 203) ends up with no documentation at all, and `row_parts`'s summary paragraph becomes the grid contract.

**Why it matters**: The public entry point that units.rs and external callers use shows no docs, and its stated contract (3/2/1 cards per row thresholds, 34-column minimum) is invisible where it applies. API-14 requires a summary on every public item.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 row_parts is moved (or its doc split) so the grid-layout doc block sits directly on layout_cards_in_grid_with_width
- [ ] #2 row_parts keeps only its own short doc
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.22 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
