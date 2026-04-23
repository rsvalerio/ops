---
id: TASK-0171
title: >-
  OWN-8: merge_indexmap clones every key and value even when overlay could be
  consumed by value
status: To Do
assignee: []
created_date: '2026-04-22 21:24'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - OWN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/core/src/config/merge.rs:13-22

**What**: merge_indexmap takes overlay: Option<&IndexMap<K,V>> and iterates with base.insert(k.clone(), v.clone()). Every call site (merge_config in the same file) owns the overlay IndexMap inside a ConfigOverlay we just deserialized and will drop immediately after merging — there is no second reader. Taking overlay: Option<IndexMap<K,V>> and using into_iter() would skip the per-entry clones entirely (and drop the K: Clone / V: Clone bounds for this helper).

**Why it matters**: OWN-8. For the Commands/Themes/Tools overlays this is N * 2 heap allocations per config load — not hot, but the clone-to-satisfy-borrow-checker smell is present: the helper is borrow-shaped only because merge_config wants to run multiple steps over &ConfigOverlay. Destructuring ConfigOverlay (which merge_config already does on line 37-46) by value would also remove the overlay.as_ref() dance on lines 51/57/71.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change merge_indexmap to take the overlay by value and use into_iter()
- [ ] #2 Adjust merge_config to consume ConfigOverlay where possible (or keep &ConfigOverlay and pass owned maps through clone() at the boundary — the point is to clone once, not per-entry)
<!-- AC:END -->
