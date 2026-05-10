---
id: TASK-0658
title: >-
  OWN-1: composite_tree_has_parallel allocates Strings instead of borrowing &str
  from Config
status: Done
assignee:
  - TASK-0741
created_date: '2026-04-30 05:12'
updated_date: '2026-04-30 19:33'
labels:
  - code-review-rust
  - idioms
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:293-312`

**What**: DFS over composite tree allocates `String` clones for every node visited (`current.clone()` into `visited`, `child.clone()` for stack push) when `&str` references into the `Config` would suffice. `Config` outlives the walk.

**Why it matters**: Per-invocation cost is small; flagged as a readability/owning-design smell rather than a perf finding.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Borrow names through the walk (HashSet<&str>, Vec<&str>); the runner's resolve already accepts &str
<!-- AC:END -->
