---
id: TASK-0211
title: >-
  FN-9: PlanLifecycle::finish accepts &[StepResult] only to derive success from
  iter().all
status: To Do
assignee: []
created_date: '2026-04-23 06:32'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - function-design
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:67`

**What**: The finish method takes the full results slice just to compute a bool; callers already own the slice.

**Why it matters**: Implicit dependency through a larger-than-needed parameter obscures that only a single bool is consumed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Change finish to accept success: bool computed by the caller
- [ ] #2 Update call sites in run_plan and run_plan_parallel accordingly
<!-- AC:END -->
