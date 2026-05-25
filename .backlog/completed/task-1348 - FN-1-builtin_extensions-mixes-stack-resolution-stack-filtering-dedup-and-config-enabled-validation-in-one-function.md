---
id: TASK-1348
title: >-
  FN-1: builtin_extensions mixes stack resolution, stack filtering, dedup, and
  config-enabled validation in one function
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-12 16:42'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - structure
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/discovery.rs:90-139`

**What**: `builtin_extensions` interleaves four concerns: (1) stack resolution (line 94), (2) compiled-in collection and stack filtering (lines 95-103), (3) dedup of compiled extensions (line 104), and (4) `config.extensions.enabled` validation + reordering (lines 112-136). The validation loop both builds the diagnostic state and bails on the first miss, and the final ordering is a second pass over the same map.

**Why it matters**: The function is the natural seam for unit testing each filter pass independently; today only end-to-end behaviour is testable, which is why TASK-1314 / TASK-1315 had to file vacuous-property tests for parts of it. Splitting into `filter_by_stack(...) -> BTreeMap` and `select_enabled(BTreeMap, &[String]) -> Result<Vec>` would let each policy carry its own test surface and prepares the ground for aggregating multiple missing-enabled names (the existing API-1 task TASK-1328 wants exactly that).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 builtin_extensions decomposed so stack filtering and enabled validation are separate, individually testable helpers
- [ ] #2 Existing behaviour (error message, ordering) preserved; cargo test --workspace passes
<!-- AC:END -->
