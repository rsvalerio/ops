---
id: TASK-0113
title: >-
  TEST-17: run_plan_parallel_executes_concurrently asserts wall-clock < 1.8s on
  two 1s sleeps
status: Done
assignee: []
created_date: '2026-04-19 18:36'
updated_date: '2026-04-19 20:29'
labels:
  - rust-code-review
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/tests.rs:1020-1046`

**What**: The test spawns two real `sleep 1` subprocesses and asserts `elapsed < 1.8s`, giving only ~0.8s of headroom above the theoretical 1.0s. On a loaded CI host this margin is likely to flake (TEST-17 pattern: timing-dependent assertion without synchronization primitives).

**Why it matters**: A false-failing concurrency test erodes trust in the suite and blocks merges. The parallelism property can be verified without a wall-clock threshold (e.g., observe that both tasks reach a rendezvous barrier before either completes).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace the wall-clock assertion with a deterministic sync point (barrier, channel, or Notify) that proves both tasks start before either finishes
- [ ] #2 Test still fails if the plan runs serially
- [ ] #3 No reliance on fixed sleep durations for correctness
<!-- AC:END -->
