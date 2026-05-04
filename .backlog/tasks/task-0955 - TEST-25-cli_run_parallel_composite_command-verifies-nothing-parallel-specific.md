---
id: TASK-0955
title: 'TEST-25: cli_run_parallel_composite_command verifies nothing parallel-specific'
status: Triage
assignee: []
created_date: '2026-05-04 21:45'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `tests/integration.rs:223-248`

**What**: The "parallel" composite test asserts the same predicate as the sequential composite test — `success()` + stderr contains "Done in". It does not verify children actually ran concurrently (overlapping windows, interleaved output, or that `parallel = true` was honoured rather than ignored). A regression that silently turns parallel into sequential passes.

**Why it matters**: TEST-25 (no project-logic test) and TEST-12 (redundant with sequential variant). The test's sole purpose — proving parallel execution — is not exercised.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test verifies an observable parallel-only property (e.g. combined wall-time < sum of sleeps, interleaved output, or both children write a side-channel file before either exits)
- [ ] #2 Or move parallel-scheduling coverage to a unit test asserting the scheduler invokes both before either finishes
<!-- AC:END -->
