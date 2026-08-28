---
id: TASK-1921
title: >-
  CONC-3: the biased select in handle_parallel_events_with_cancel_inner starves
  the JoinSet arm, so a panicked task does not trip fail_fast while any sibling
  is still emitting output
status: To Do
assignee:
  - TASK-1986
created_date: '2026-08-27 15:44'
updated_date: '2026-08-28 14:10'
labels:
  - code-review-rust
  - concurrency
dependencies: []
modified_files:
  - crates/runner/src/command/parallel.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/parallel.rs:555-593` (`handle_parallel_events_with_cancel_inner`)

**What**: the drain loop races the event channel against the `JoinSet` under `biased`:

```rust
tokio::select! {
    biased;
    ev = rx.recv(), if rx_open => { ... }
    joined = join_set.join_next_with_id(), if !join_set.is_empty() => { ... }
}
```

`biased` removes the random polling order and makes `select!` poll arms strictly top-to-bottom, returning as soon as one is ready. The second arm is therefore only reached on an iteration where `rx.recv()` is *pending*. That is exactly the condition that does not hold for the workload this loop exists to serve: a parallel plan of chatty steps keeps the bounded mpsc permanently non-empty (each `exec_standalone` forwarder is pushing a `StepOutput` per captured line), so `rx.recv()` is ready on every poll and `join_next_with_id` is never polled.

That silently defeats the contract the arm was added for. The doc comment directly above it (CONC-6 / TASK-1177) states the goal: "a task that **panics** (rather than returning a non-zero exit code) trips `fail_fast` at the same point a `StepFailed` event would. Pre-fix, a panicked sibling surfaced only after the channel drained naturally — defeating `fail_fast` for the panic path while a 5-second sibling kept emitting output." Under `biased` the post-fix behaviour is the same as the pre-fix behaviour in precisely that scenario: the panic is observed only once the noisy sibling goes quiet or the channel closes.

The `biased` keyword is load-bearing elsewhere in this crate (`exec.rs:625`, `exec.rs:651`) where the intent really is "prefer the abort arm", and it looks like it was copied here without that intent. Here the two arms are peers and want fair polling: dropping `biased` restores `select!`'s randomised order, which gives the JoinSet arm a chance on every iteration. If a deterministic order is wanted instead, alternate explicitly (e.g. a parity flag, or drain the JoinSet non-blockingly with `try_join_next` before each `select!`).

Note the ordering is not protecting event delivery either: `abort_all()` in the JoinSet arm is the same call the `StepFailed` arm makes, and events already in the channel are still drained by the `!rx_open && join_set.is_empty()` exit condition.

**Why it matters**: `fail_fast` is a wall-clock promise — stop the plan the moment something breaks. A panicking step (an `unwrap` in a future refactor of `exec_standalone`, a JoinSet-internal invariant, an OOM-abort in a spawned drain) is the one failure class that produces no `StepFailed` event, so this arm is its only trigger. Today it fires last instead of first, and only in the quiet case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 the JoinSet arm is polled on iterations where the event channel is continuously ready — either by dropping 'biased' or by an explicit fair/alternating scheme
- [ ] #2 if 'biased' is kept for a deliberate reason, that reason is documented and shown to be compatible with the CONC-6 / TASK-1177 contract quoted in the doc comment
- [ ] #3 a regression test runs a plan under fail_fast where one task panics while a sibling floods StepOutput, and asserts the sibling is aborted while the flood is still in flight (not merely that the panic result eventually appears)
<!-- AC:END -->
