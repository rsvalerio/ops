---
id: TASK-0539
title: >-
  PERF-1: ProgressState::record_stderr accumulates unbounded stderr lines per
  step
status: Done
assignee:
  - TASK-0643
created_date: '2026-04-29 04:58'
updated_date: '2026-04-29 14:22'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/display/progress_state.rs:72`

**What**: `record_stderr` pushes every captured stderr line into `step_stderr[id]: Vec<String>` with no upper bound. Even with the 4 MiB CommandOutput cap (TASK-0515), the per-line split and Vec<String> retention happens for the whole plan duration only to surface the last `stderr_tail_lines` (default 5) on failure.

**Why it matters**: A noisy build (`cargo test --all-features`, `--verbose` runs) keeps millions of small String allocations resident when only the tail is ever consumed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 record_stderr maintains a bounded ring (e.g. VecDeque sized to max(stderr_tail_lines, verbose-cap) per id) so peak memory is O(tail), not O(captured stderr)
- [x] #2 --verbose mode preserves today's full-tail rendering by raising the cap or bypassing the ring, with a regression test for a 100k-line stderr stream
<!-- AC:END -->
