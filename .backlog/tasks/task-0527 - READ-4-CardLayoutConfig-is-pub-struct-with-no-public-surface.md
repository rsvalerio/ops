---
id: TASK-0527
title: 'READ-4: CardLayoutConfig is pub struct with no public surface'
status: Done
assignee:
  - TASK-0534
created_date: '2026-04-28 06:52'
updated_date: '2026-04-28 18:51'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:15`

**What**: `pub struct CardLayoutConfig;` exposes the type publicly but every associated constant (CARD_WIDTH, CARD_DESC_LINES, CARD_SPACING, MIN_WIDTH_*) is private. Out-of-crate code can name the type but read nothing on it.

**Why it matters**: Dormant API noise: the `pub` is misleading. Either the constants should be public (so downstream consumers can compose layouts) or the struct should be private.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either make the struct private or expose the constants as pub const
- [ ] #2 Update tests/docs to show the intended use
<!-- AC:END -->
