---
id: TASK-0766
title: >-
  PERF-3: expand_inner does two store lookups per node (canonical_id then
  resolve)
status: Triage
assignee: []
created_date: '2026-05-01 05:55'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/resolve.rs:128-133`

**What**: After canonical_id(id) traverses config → stack → extension → alias maps to find the canonical key, resolve(canonical) immediately re-traverses the same maps to fetch the spec. Recursion-heavy composites pay 2x map lookups per node.

**Why it matters**: Composite expansion runs on every `ops <cmd>` invocation; duplicate work scales with composite-graph size and is straightforward to avoid by returning (canonical, spec) from a single helper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a helper that returns (canonical_name, spec) in one traversal, replacing the canonical_id+resolve pair in expand_inner
- [ ] #2 Keep the public canonical_id/resolve signatures intact (callers in tests rely on them)
- [ ] #3 Bench or microbench shows expand_to_leaves is at least as fast as before on a representative composite
<!-- AC:END -->
