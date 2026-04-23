---
id: TASK-0191
title: >-
  READ-4: CardLayoutConfig::BOX_WIDTH is pub without documented external
  consumer
status: To Do
assignee: []
created_date: '2026-04-22 21:26'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:16-30`

**What**: CardLayoutConfig::BOX_WIDTH is pub const = 100 while CARD_WIDTH, CARD_DESC_LINES, CARD_SPACING, MIN_WIDTH_3_CARDS, MIN_WIDTH_2_CARDS are private. Rationale for exposing only BOX_WIDTH is not documented. No references found outside the crate.

**Why it matters**: READ-4/ARCH-9 — minimal public surface and document why decisions. Narrow to pub(crate) or add a doc comment explaining the external consumer.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 BOX_WIDTH visibility is reduced to pub(crate) or documented with an external consumer
<!-- AC:END -->
