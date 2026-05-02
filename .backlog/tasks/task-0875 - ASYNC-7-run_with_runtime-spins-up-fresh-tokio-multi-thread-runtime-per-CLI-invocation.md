---
id: TASK-0875
title: >-
  ASYNC-7: run_with_runtime spins up fresh tokio multi-thread runtime per CLI
  invocation
status: Triage
assignee: []
created_date: '2026-05-02 09:23'
labels:
  - code-review-rust
  - async
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/run_cmd.rs:86-94`

**What**: Each run_command_* path constructs tokio::runtime::Runtime::new() and block_ons the work. The runtime defaults to a multi-threaded scheduler with a worker thread per CPU, which is created and torn down every invocation even when only a single short command runs.

**Why it matters**: ASYNC-7 guidance is "use sync code when async adds no value" or use the lighter current_thread runtime when you do not need the multi-thread pool. For sequential single-command CLI runs, the worker-thread fan-out is pure overhead.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Switch to tokio::runtime::Builder::new_current_thread().enable_all().build() when any_parallel == false, retaining the multi-thread runtime for the parallel path
- [ ] #2 Measure: time ops <noop-cmd> startup shows fewer threads created (visible via pthread_create count or /proc/<pid>/status)
- [ ] #3 Existing sequential tests are unaffected
<!-- AC:END -->
