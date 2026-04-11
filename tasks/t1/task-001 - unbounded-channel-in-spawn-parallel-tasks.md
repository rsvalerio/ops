---
id: TASK-001
title: "Unbounded channel in spawn_parallel_tasks bypasses backpressure"
status: To Do
assignee: []
created_date: '2026-04-06 00:00:00'
labels: [rust-idioms, EFF, ASYNC-3, CONC-3, medium, effort-S, crate-runner]
dependencies: []
---

## Description

**Location**: `crates/runner/src/command/mod.rs:364`
**Anchor**: `fn spawn_parallel_tasks`
**Impact**: `mpsc::unbounded_channel()` is used for runner events. While comments note the parallel group is typically small (<100 commands, <1KB events), an unbounded channel provides no backpressure guarantee. If a future use case increases parallelism or event volume, memory could grow without bound.

**Notes**:
Replace `mpsc::unbounded_channel()` with `mpsc::channel(capacity)` where capacity is ~2–3× expected concurrent commands (e.g., `mpsc::channel(256)`). The `handle_parallel_events` consumer already drives the channel via `while let Some(ev) = rx.recv().await`, so switching to a bounded channel requires no consumer changes — senders will naturally pause when the buffer is full. Current risk is low given the small parallel group sizes documented in the code, but bounded channels are the idiomatic default per ASYNC-3 and CONC-3.
