---
id: TASK-1395
title: 'PERF-3: merge_output clones theme and category_order on every overlay merge'
status: Done
assignee:
  - TASK-1453
created_date: '2026-05-13 18:06'
updated_date: '2026-05-13 20:39'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/merge.rs:47, 51`

**What**: `merge_output` takes `&OutputConfigOverlay` and calls `overlay.theme.clone()` and `overlay.category_order.clone()` to satisfy `merge_field<T>(_, Option<T>)`. The cloned `Option<Vec<String>>` and theme allocate on every config layer merge even though the overlay is consumed immediately afterward.

**Why it matters**: The caller (`merge_config_overlay`) already owns or destructures the overlay; switching `merge_output` to consume the overlay by value (or destructure it) lets the inner Vec/String be moved instead of cloned, eliminating two allocations per merge with no behavioral change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 merge_output is restructured to move (not clone) theme and category_order
- [ ] #2 Existing merge tests pass and no new clones are introduced elsewhere
<!-- AC:END -->
