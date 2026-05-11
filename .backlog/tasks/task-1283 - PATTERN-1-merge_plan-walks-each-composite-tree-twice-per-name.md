---
id: TASK-1283
title: 'PATTERN-1: merge_plan walks each composite tree twice per name'
status: To Do
assignee:
  - TASK-1305
created_date: '2026-05-11 15:26'
updated_date: '2026-05-11 16:48'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/plan.rs:37-47`

**What**: Inside the names loop, `runner.expand_to_leaves(name)` already walks the composite tree to collect leaves, then `composite_tree_flags(runner, name)` walks the same tree again to determine `any_parallel` / `fail_fast_disabled`. Both walks visit identical nodes via independent traversal logic.

**Why it matters**: For deep composite trees, runtime doubles for no behavioural reason. More importantly the two independent walks can drift in resolution semantics (different cycle handling, different child ordering) and yield plan flags that disagree with the actually-executed leaf set — the exact bug class PATTERN-1 / TASK-0754 was introduced to prevent.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single traversal that returns (leaf_ids, has_parallel, fail_fast_disabled) per name
- [ ] #2 Existing nested-parallel and nested-fail_fast tests in tests.rs still pass
- [ ] #3 No remaining double-walk over the same composite subtree in the run path
<!-- AC:END -->
