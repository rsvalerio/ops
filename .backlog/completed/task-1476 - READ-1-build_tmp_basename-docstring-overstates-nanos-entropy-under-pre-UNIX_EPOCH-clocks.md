---
id: TASK-1476
title: >-
  READ-1: build_tmp_basename docstring overstates nanos entropy under
  pre-UNIX_EPOCH clocks
status: Done
assignee:
  - TASK-1482
created_date: '2026-05-16 10:06'
updated_date: '2026-05-17 08:56'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/edit.rs:156-159`

**What**: `SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos())` returns 0 when the clock has been set before 1970. Combined with the pid/counter the tmp name is still unique within the process, but the docstring claim ("unique per (process, monotonic counter, nanos)") is no longer accurate in that edge case.

**Why it matters**: Honesty in comments matters; a future reader will assume nanos contributes entropy and a future change might drop the counter on the assumption that nanos suffices.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Use Instant::now().elapsed() (monotonic) or fall back to the counter alone; update the comment to match
- [ ] #2 Or: keep the wall-clock formatting but rewrite the comment to call out that nanos is best-effort and uniqueness is carried by (pid, counter)
<!-- AC:END -->
