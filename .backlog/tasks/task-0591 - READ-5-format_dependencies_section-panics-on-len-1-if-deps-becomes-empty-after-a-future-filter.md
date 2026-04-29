---
id: TASK-0591
title: >-
  READ-5: format_dependencies_section panics on len() - 1 if deps becomes empty
  after a future filter
status: Done
assignee:
  - TASK-0644
created_date: '2026-04-29 05:18'
updated_date: '2026-04-29 17:05'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/deps.rs:60`

**What**: After filtering empty units, the inner loop computes `let last_idx = unit.deps.len() - 1;`. The current outer filter guarantees deps is non-empty, but a future refactor that adds another filter and lets an empty deps slice through will panic on `0usize - 1`.

**Why it matters**: READ-5. The invariant is enforced 12 lines earlier and re-violated trivially by future edits. Replace with `.enumerate().peekable()` or `.split_last()`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 last_idx computation replaced with pattern that does not subtract from possibly-zero length
- [x] #2 Regression test passes a unit with empty deps directly
<!-- AC:END -->
