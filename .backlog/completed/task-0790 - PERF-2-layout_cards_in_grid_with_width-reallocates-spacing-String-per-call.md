---
id: TASK-0790
title: 'PERF-2: layout_cards_in_grid_with_width reallocates spacing String per call'
status: Done
assignee:
  - TASK-0828
created_date: '2026-05-01 05:58'
updated_date: '2026-05-02 08:18'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:191`

**What**: `let spacing = " ".repeat(CardLayoutConfig::CARD_SPACING);` allocates a fresh String on every grid layout call. CARD_SPACING is a compile-time constant (2), so spacing is always "  ".

**Why it matters**: Trivial, but the function is called per render of every about-units page; a `const SPACING: &str = "  ";` removes the allocation entirely.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace runtime repeat with a compile-time &'static str constant
- [ ] #2 Confirm benchmark is unchanged or improved
<!-- AC:END -->
