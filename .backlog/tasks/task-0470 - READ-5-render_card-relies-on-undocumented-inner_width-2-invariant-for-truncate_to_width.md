---
id: TASK-0470
title: >-
  READ-5: render_card relies on undocumented inner_width >= 2 invariant for
  truncate_to_width
status: To Do
assignee:
  - TASK-0534
created_date: '2026-04-28 05:47'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:74`

**What**: render_card calls truncate_to_width(&title, inner_width) where inner_width = CARD_WIDTH - 2 = 30. truncate_to_width's loop uses max_width.saturating_sub(1) as the threshold; for max_width == 0 it produces a single ellipsis. CARD_WIDTH is currently 32 (safe), but layout_cards_in_grid_with_width has no minimum-width guard, and a future change to CARD_WIDTH < 4 would silently produce a card of pure ellipses.

**Why it matters**: Defensive readability — the implicit invariant should be documented or compile-time asserted.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 const _: () = assert!(CardLayoutConfig::CARD_WIDTH >= 4); (or equivalent compile-time check) added to cards.rs
- [ ] #2 Doc-comment on CARD_WIDTH explicitly states the >= 4 minimum and why
<!-- AC:END -->
