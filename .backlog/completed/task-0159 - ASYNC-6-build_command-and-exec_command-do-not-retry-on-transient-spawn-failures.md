---
id: TASK-0159
title: >-
  ASYNC-6: build_command and exec_command do not retry on transient spawn
  failures
status: Done
assignee: []
created_date: '2026-04-22 21:23'
updated_date: '2026-04-23 15:06'
labels:
  - rust-code-review
  - ASYNC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:352-384` (exec_command)

**What**: `exec_command` wraps `cmd.output()` with `run_with_timeout` but does not implement retries with exponential backoff. Transient failures (e.g. EAGAIN from fork under load, temporary PATH resolution hiccup, flaky network filesystem on `current_dir`) produce an immediate failure with no retry. ASYNC-6 recommends "Timeouts + retries + exponential backoff for all external calls."

**Why it matters**: In CI environments (heavy parallel load, containerized FS), transient spawn failures surface as step failures that require the user to retry the whole plan. For commands that are idempotent at the exec level (they haven't run yet), a short retry on `io::ErrorKind` that indicates transience (Interrupted, WouldBlock, ResourceBusy) would meaningfully reduce flakiness without changing success semantics. Note: retry must be gated to pre-spawn errors only; never re-spawn after a process has exited non-zero.

<!-- scan confidence: design suggestion, not a definite bug -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Decide whether pre-spawn transient errors warrant a single retry; document the decision in exec.rs
<!-- AC:END -->
