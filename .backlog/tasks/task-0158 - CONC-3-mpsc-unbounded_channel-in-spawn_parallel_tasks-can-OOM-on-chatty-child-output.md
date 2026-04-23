---
id: TASK-0158
title: >-
  CONC-3: mpsc::unbounded_channel in spawn_parallel_tasks can OOM on chatty
  child output
status: To Do
assignee: []
created_date: '2026-04-22 21:23'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - CONC
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/mod.rs:491-494`

**What**: `spawn_parallel_tasks` uses `mpsc::unbounded_channel()` to funnel `RunnerEvent::StepOutput` from every parallel subprocess into the display. The inline comment justifies this by saying "Memory is bounded by actual subprocess output which is finite" — but for a child that emits megabytes of stdout/stderr faster than the display loop can drain (e.g. `cargo build --verbose` with dep warnings, or a test suite printing per-line progress), the channel grows without bound while the display is still rendering prior events. In the worst case (many parallel children + slow terminal) memory grows until OOM.

**Why it matters**: CONC-3 anti-pattern: "Red flag: unbounded channels in production — they defer OOM to runtime instead of surfacing backpressure." The documented rationale ("events must not be dropped") is addressed by a bounded channel with backpressure (producer awaits on `send`), not by making the channel unbounded. Fix: switch to `mpsc::channel(capacity)` with a tuned capacity (start 1024), let the standalone task await on send — this naturally slows noisy children to match display throughput.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace unbounded_channel with bounded channel in spawn_parallel_tasks
- [ ] #2 Benchmark that a child emitting 10MB stdout does not balloon process RSS
<!-- AC:END -->
