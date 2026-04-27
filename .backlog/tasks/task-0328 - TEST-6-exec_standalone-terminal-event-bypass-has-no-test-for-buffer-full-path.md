---
id: TASK-0328
title: 'TEST-6: exec_standalone terminal-event bypass has no test for buffer-full path'
status: Done
assignee:
  - TASK-0414
created_date: '2026-04-26 08:51'
updated_date: '2026-04-26 10:27'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:321-350`

**What**: A new branch was added to `exec_standalone` that captures `StepFinished/StepFailed/StepSkipped` events from the `exec_command` callback into a local `Option<RunnerEvent>` and forwards them on the *outer* `tx` (with awaited backpressure) after the local-buffer forwarder has drained. This was introduced precisely to fix a real bug: under noisy commands, the 256-slot local buffer fills up and `try_send` drops events — and if the dropped event was the *terminal* one, the display's progress bar is orphaned forever.

The accompanying tests only cover the orphan-bar finalization on the display side (`display/tests.rs::run_finished_finalizes_orphan_running_bars`). The exec-side fix has no test:

- `command/tests.rs::exec_standalone_skips_when_abort_set` is the only `exec_standalone` test and exercises the early-abort path, not the terminal-event bypass.
- No test asserts that, when the local 256-slot buffer is saturated by a flood of `StepOutput` events, the trailing `StepFinished/StepFailed/StepSkipped` is still delivered on the outer channel.
- No test asserts ordering: that the terminal event is delivered *after* all forwarded output events, not interleaved/before.

**Why it matters**: this is exactly the kind of regression the fix is meant to prevent. Without a test, a future refactor (e.g. removing the post-drain `tx.send`, reordering relative to `forwarder.await`, switching back to a single-path `try_send`) can silently reintroduce the orphan-bar bug — caught only by a human running `cargo test --all-features` and noticing a stuck spinner. TEST-6 (error/edge paths) and TEST-7 (each new branch needs at least one test) both apply.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add a test in crates/runner/src/command/tests.rs that drives exec_standalone with a callback-emitting workload large enough to fill the 256-slot LOCAL_BUF, and asserts the terminal event (StepFinished or StepFailed) is observed on the outer rx
- [ ] #2 Test asserts the terminal event arrives after all non-dropped StepOutput events forwarded from the buffer (no out-of-order delivery)
- [ ] #3 Test runs under #[tokio::test(flavor = "multi_thread")] so the forwarder task and the producer race realistically
- [ ] #4 Test does not rely on sleep-based synchronization (TEST-15); use channel/Notify-based deterministic sync
<!-- AC:END -->
