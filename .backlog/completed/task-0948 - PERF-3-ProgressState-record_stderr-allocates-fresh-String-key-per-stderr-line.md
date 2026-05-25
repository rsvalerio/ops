---
id: TASK-0948
title: >-
  PERF-3: ProgressState::record_stderr allocates fresh String key per stderr
  line
status: Done
assignee: []
created_date: '2026-05-04 21:33'
updated_date: '2026-05-04 22:47'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/progress_state.rs:93`

**What**: `step_stderr.entry(id.to_string()).or_default()` allocates a fresh `String` on every stderr line, even when the entry already exists.

**Why it matters**: Under noisy commands (e.g. `cargo test --all-features`) emitting tens of thousands of stderr lines, this per-line heap allocation is unnecessary churn on the display hot path. A `get_mut`-then-`entry` two-step skips allocation on the common Occupied case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 record_stderr performs at most one id.to_string() allocation per (plan, step id); subsequent calls for the same id reuse the existing key
- [ ] #2 Behaviour for unknown ids and the cap == 0 short-circuit is preserved; existing tests record_stderr_accumulates_per_id and record_stderr_bounded_ring_keeps_only_tail pass
<!-- AC:END -->
