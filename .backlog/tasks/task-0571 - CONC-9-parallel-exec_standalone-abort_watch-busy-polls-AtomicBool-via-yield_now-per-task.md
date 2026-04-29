---
id: TASK-0571
title: >-
  CONC-9: parallel exec_standalone abort_watch busy-polls AtomicBool via
  yield_now per task
status: Triage
assignee: []
created_date: '2026-04-29 05:16'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:376`

**What**: In `exec_standalone`, both the forwarder task (lines 363-385) and the terminal-event delivery (lines 446-462) implement `abort_watch` as `while !abort.load(Acquire) { tokio::task::yield_now().await; }`. Under `MAX_PARALLEL=32` there are up to 64 such loops live simultaneously, each waking the executor on every poll cycle.

**Why it matters**: CONC-3/CONC-9: proper primitive is `tokio::sync::Notify` / `watch` / `CancellationToken`. The loop burns CPU on the runtime, inflates wakeup latency for real I/O, and pollutes flamegraphs. TASK-0459 closed the deadlock; this is the resource-quality follow-up.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Abort signal exposed as Notify/watch::Receiver/CancellationToken instead of Arc<AtomicBool>
- [ ] #2 abort_watch async block in exec.rs replaced with single await on the notification future
- [ ] #3 Both call sites (forwarder ~376 and terminal-event ~446) updated together
- [ ] #4 Existing parallel-cancellation tests still pass
<!-- AC:END -->
