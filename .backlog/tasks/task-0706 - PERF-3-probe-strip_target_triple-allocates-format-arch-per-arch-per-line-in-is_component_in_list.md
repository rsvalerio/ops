---
id: TASK-0706
title: >-
  PERF-3: probe::strip_target_triple allocates format!('-{arch}-') per arch per
  line in is_component_in_list
status: To Do
assignee:
  - TASK-0741
created_date: '2026-04-30 05:29'
updated_date: '2026-04-30 06:07'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/probe.rs:235-242`

**What**: `strip_target_triple` iterates the 30-entry `RUSTUP_TARGET_ARCHES` table and for each arch builds a fresh `format!("-{arch}-")` String, then calls `line.find(&...)`. `is_component_in_list` runs this on every line of `rustup component list --installed` for every component probed by `check_tool_status`. The allocation count is `30 * lines * components` — a workspace with 6 tools and a typical 20-line rustup output churns ~3,600 short-lived String allocations per probe.

**Why it matters**: ops about / ops about-tools runs probes in the foreground, so allocation churn is on the user-perceived path. The fix is mechanical: precompute the prefix patterns into `&'static str` constants (`"-aarch64-"`, `"-arm-"`, ...) at module scope and let `line.find(&str)` walk them without allocation. PERF-3 territory: dormant, easy to fix, no behaviour change.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 strip_target_triple does not allocate per arch per line — patterns live as &\'static str at module scope
- [ ] #2 is_component_in_list passes a microbench / inspection showing zero allocations on the prefix matching path
- [ ] #3 behaviour-equivalence test (existing TASK-0560 fixtures) still passes
<!-- AC:END -->
