---
id: TASK-0505
title: 'PATTERN-1: expand_inner false-positive cycle on diamond composite topology'
status: To Do
assignee:
  - TASK-0537
created_date: '2026-04-28 06:50'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - correctness
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/resolve.rs:135`

**What**: `visited` is a single shared HashSet inserted-but-never-removed. If two sibling composites both reference the same composite child, the second visit raises ExpandError::Cycle even though the graph is a DAG.

**Why it matters**: A legitimate diamond composite layout (e.g. A -> [B,C]; B -> [D]; C -> [D]; D = Composite) currently fails to expand with a misleading "cycle detected" error. Cycle detection should track the active recursion stack, not all-time visits.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Construct a diamond DAG of composites and assert expand_to_leaves succeeds
- [ ] #2 Insert/remove canonical id only along the active path (DFS in/out)
- [ ] #3 Keep cycle test for true self-reference green
<!-- AC:END -->
