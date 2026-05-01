---
id: TASK-0754
title: >-
  PATTERN-1: merge_plan only inspects top-level composite for
  parallel/fail_fast, ignoring nested composites
status: Triage
assignee: []
created_date: '2026-05-01 05:53'
labels:
  - code-review-rust
  - patterns
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/plan.rs:17`

**What**: merge_plan calls runner.resolve(name) and reads c.parallel / c.fail_fast on the top-level composite only. If name resolves to a composite whose children are themselves composites with parallel = true or fail_fast = false, those flags are silently dropped. TASK-0511 fixed the analogous bug in warn_raw_drops_parallel by walking composite_tree_has_parallel; the multi-command merge path still has the shallow check.

**Why it matters**: A user invoking `ops run umbrella` where umbrella is a composite of [parallel_inner] (and parallel_inner.parallel = true) gets sequential execution with no warning. Silent loss of parallelism / fail-fast semantics.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 merge_plan recurses into composite children (with cycle protection mirroring composite_tree_has_parallel) when computing any_parallel and fail_fast
- [ ] #2 Test pins behaviour for outer composite outer = { commands = ["inner"], parallel = false } and inner = { commands = [...], parallel = true }: merge_plan returns any_parallel = true
- [ ] #3 Symmetric test pins fail_fast aggregation across nested composites
<!-- AC:END -->
