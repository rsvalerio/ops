---
id: TASK-1130
title: >-
  PERF-3: render_separator and wrap_step_line allocate fresh space-repeat
  strings per step
status: To Do
assignee:
  - TASK-1263
created_date: '2026-05-08 07:39'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/configurable.rs:268`

**What**: `wrap_step_line` (line 268) calls `\" \".repeat(right_pad)` and `render_separator` (lines 340, 343) calls `sep.to_string().repeat(...)` once per rendered step line. The render path already precomputes `left_pad_str` at construction (line 48) under PERF-3 / TASK-1035, but per-step right-pad and separator-fill strings still allocate.

**Why it matters**: These functions are on the per-step render hot path under boxed layout, invoked once per `StepLine` render, so allocations scale with O(steps_per_run).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Avoid temporary String allocations in wrap_step_line and render_separator by pushing chars directly into output buffer
- [ ] #2 Add a test pinning the no-extra-allocation contract on the hot path
<!-- AC:END -->
