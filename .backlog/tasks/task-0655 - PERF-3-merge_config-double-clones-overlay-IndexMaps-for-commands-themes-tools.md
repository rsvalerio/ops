---
id: TASK-0655
title: 'PERF-3: merge_config double-clones overlay IndexMaps for commands/themes/tools'
status: To Do
assignee:
  - TASK-0741
created_date: '2026-04-30 05:12'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/merge.rs:63,65,74`

**What**: `merge_config` calls `commands.clone()` / `themes.clone()` / `tools.clone()` on the overlay IndexMaps before passing to `merge_indexmap`, even though the overlay is consumed by `&ConfigOverlay`. `merge_indexmap` then `extend`s and clones every key/value out of that throwaway — two clones per overlay where one suffices.

**Why it matters**: Cost is bounded but the API is misleading: the "overlay-by-reference" signature hides that internally the merge owns the data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Take the overlay by value (merge_config(base: &mut Config, overlay: ConfigOverlay)) and move the IndexMaps in, or change merge_indexmap to accept &IndexMap and extend from references with explicit clones at the leaf only
<!-- AC:END -->
