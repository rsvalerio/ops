---
id: TASK-1091
title: >-
  PATTERN-1: merge_plan returns fail_fast=true for an empty names slice, masking
  a likely caller bug
status: Done
assignee: []
created_date: '2026-05-07 21:31'
updated_date: '2026-05-08 06:35'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd/plan.rs:13-32`

**What**: `merge_plan` initialises `fail_fast = true` and never updates it for an empty `names`. An empty slice represents either an upstream filtering bug or an unintended caller invariant; returning `(empty_plan, false, true)` lets callers proceed silently. The plan executor then runs zero steps and reports success.

**Why it matters**: Silent "ran nothing, success" is the worst failure mode for a runner — masks upstream filtering bugs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Either return an error for an empty names slice, or document the empty-slice contract explicitly and audit callers
- [x] #2 Add a debug-build assertion or a unit test pinning the chosen behaviour
- [x] #3 Verify all production call sites either guarantee non-empty slices or handle the empty plan
<!-- AC:END -->
