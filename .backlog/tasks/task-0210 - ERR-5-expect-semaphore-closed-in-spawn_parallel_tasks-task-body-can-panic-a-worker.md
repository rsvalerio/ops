---
id: TASK-0210
title: >-
  ERR-5: expect("semaphore closed") in spawn_parallel_tasks task body can panic
  a worker
status: Done
assignee: []
created_date: '2026-04-23 06:32'
updated_date: '2026-04-23 14:59'
labels:
  - rust-code-review
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:507`

**What**: The semaphore is owned by the JoinSet's parent scope and an `expect` panics inside the spawned task if the semaphore is ever dropped early.

**Why it matters**: Panic is caught by collect_join_results, but it yields a <panicked> StepResult rather than a clear error.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace .expect("semaphore closed") with a match returning a StepResult::failure with descriptive message
- [ ] #2 Add a unit test that forces the semaphore-closed branch or document why unreachable
<!-- AC:END -->
