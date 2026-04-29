---
id: TASK-0590
title: >-
  READ-5: layout_cards_in_grid_with_width does not narrow CARD_WIDTH for very
  small terminals
status: Triage
assignee: []
created_date: '2026-04-29 05:18'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:156`

**What**: layout_cards_in_grid_with_width chooses 1/2/3 cards per row based on term_width thresholds (105/70) but always renders each card at fixed CARD_WIDTH = 32. When term_width < 32 (e.g. narrow tmux pane, COLUMNS=24), single-card mode produces 32-wide rows that overflow and corrupt layout.

**Why it matters**: READ-5 — contract "render adapts to terminal width" is implicit but breaks at the per-card level. Either narrow CARD_WIDTH with truncate_to_width (TASK-0470), or document a minimum and clamp.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Sub-32-column terminals produce documented degraded render or use narrower CARD_WIDTH
- [ ] #2 Unit test pins behavior at term_width = 24
- [ ] #3 Render contract documented on layout_cards_in_grid_with_width
<!-- AC:END -->
