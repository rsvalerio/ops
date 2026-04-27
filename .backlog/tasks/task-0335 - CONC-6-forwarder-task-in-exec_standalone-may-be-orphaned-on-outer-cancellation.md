---
id: TASK-0335
title: >-
  CONC-6: forwarder task in exec_standalone may be orphaned on outer
  cancellation
status: Done
assignee:
  - TASK-0414
created_date: '2026-04-26 09:33'
updated_date: '2026-04-26 10:57'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:314-320,347`

**What**: `exec_standalone` `tokio::spawn`s a forwarder task and only awaits its handle after `exec_command` returns. If the outer task is aborted (fail_fast abort_all), the JoinHandle is dropped without aborting the forwarder.

**Why it matters**: With high parallel-command counts and frequent fail_fast triggers, transient orphan forwarders accumulate. Today they exit promptly because local_tx drops with the future, but the lifetime is implicit and brittle to refactoring.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use a JoinSet or scope the forwarder so cancellation explicitly aborts it
- [ ] #2 Add a test that aborts exec_standalone mid-flight and asserts no spawned task remains pending after the parent future is dropped
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Wave 25 actually closed: structural fix using JoinSet for forwarder lifetime, plus the AC#2 abort-mid-flight test (exec_standalone_aborts_forwarder_on_outer_cancellation) which uses outer-channel closure as the deterministic observable — under the buggy bare-spawn version that test would hang.
<!-- SECTION:NOTES:END -->
