---
id: TASK-0422
title: 'READ-4: AboutCard::render accepts unused _columns parameter'
status: Done
assignee:
  - TASK-0534
created_date: '2026-04-28 04:41'
updated_date: '2026-04-28 18:44'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:124`

**What**: `AboutCard::render(&self, _columns: u16, is_tty: bool)` accepts a `_columns` parameter that is never used in the body. The sole call site in `extensions/about/src/lib.rs:96` faithfully computes and passes a column count that is then discarded.

**Why it matters**: Same pattern as the already-filed TASK-0281 (`render_plan_header` accepting `_columns`) — dead parameters mislead callers into thinking width affects rendering and force them to compute and thread a value with no effect, which becomes a maintenance hazard if/when responsive layout is wanted later.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either remove the _columns parameter from AboutCard::render and update the about extension call site, or make the parameter used (e.g. wrap long values to width)
- [ ] #2 No production caller passes a value that is then thrown away
- [ ] #3 Tests still pass
<!-- AC:END -->
