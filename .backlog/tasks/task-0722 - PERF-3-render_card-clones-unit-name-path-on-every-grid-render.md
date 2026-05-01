---
id: TASK-0722
title: 'PERF-3: render_card clones unit name/path on every grid render'
status: Done
assignee:
  - TASK-0741
created_date: '2026-04-30 05:31'
updated_date: '2026-04-30 19:38'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/cards.rs:80`

**What**: `render_card` repeatedly clones owned String data from `&ProjectUnit`: `unit.name.clone()` (when there is no version), the `title.clone()` short-path, `unit.path.clone()` short-path, and `pad_to_width_plain(&title_truncated, ...)` returning a fresh String. Every call also constructs a new `empty_line` String and calls `format!("{}{}{}", ...)` for every card line. `layout_cards_in_grid_with_width` then does `line.to_string()` per cell again, so each line allocates twice for what could be a borrow.

**Why it matters**: PERF-3 / OWN-8: cloning to satisfy ownership when the data is read-only. Card rendering happens on every `about` invocation, but more importantly every `about units` invocation renders N cards (one per workspace member). For large monorepos N is in the dozens or hundreds and the constant overhead is real. Switching the helpers to take `&str` and using a single `String::with_capacity` per card cuts the allocation count meaningfully without changing the contract.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Audit render_card for redundant clone()/to_string() calls — most can be replaced by &str borrows or a single preallocated String
- [x] #2 layout_cards_in_grid_with_width: avoid the per-cell line.to_string() — push &str into the formatted row
- [x] #3 Bench the hot path on a workspace with 100+ units to confirm the reduction
<!-- AC:END -->
