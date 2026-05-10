---
id: TASK-0656
title: >-
  CL-3: ProgressDisplay::handle_event sync-IO contract enforced only by doc
  comment
status: Done
assignee:
  - TASK-0742
created_date: '2026-04-30 05:12'
updated_date: '2026-04-30 20:03'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display.rs:171-176`

**What**: `handle_event` performs synchronous `write(2)` on tap/stderr inside an otherwise async-driven runner; the invariant "must only be driven from a synchronous event-pump loop, never polled inside a tokio task" lives only in a doc comment.

**Why it matters**: Any future caller who wires `handle_event` into a `tokio::spawn` (the obvious refactor) silently re-introduces sync-IO-on-runtime-thread (CONC-5). Doc-only invariants on a public-ish method are exactly the API fragility ARCH-2 / type-safety guidance flags.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Encode the constraint structurally (e.g. take &mut self from a !Send wrapper via PhantomData<*const ()>) so it cannot be moved into a tokio::spawn task; or rename to make the contract obvious (handle_event_sync)
- [ ] #2 Add a compile-fail doc test for the spawn case
<!-- AC:END -->
