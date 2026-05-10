---
id: TASK-0725
title: >-
  CONC-5: has_staged_files_with_timeout busy-polls try_wait every 50ms in
  pre-commit hot path
status: Done
assignee:
  - TASK-0735
created_date: '2026-04-30 05:47'
updated_date: '2026-04-30 06:19'
labels:
  - code-review-rust
  - CONC
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:125-148`

**What**: `has_staged_files_with_timeout` enforces its bound by busy-polling `child.try_wait()` in a `loop { thread::sleep(50ms) }` pattern. This is the sync analogue of the pattern fixed in TASK-0451 / TASK-0300 / TASK-0372 for run_with_timeout. The function is called on every developer commit (the pre-commit hook hot path) so the wakeups happen on every commit. A `wait-timeout` crate call (or a `pidfd`/`waitid`-based primitive on unix, kqueue on macOS) would block until either child exit or the deadline without spinning the scheduler.

**Why it matters**: Wasted scheduler wakeups on the developer critical path; also a 50ms granularity means a fast `git diff --cached` (typical: <5ms) still pays one unconditional `thread::sleep(50ms)` before observing exit. Net effect on a noop commit is ~50ms extra latency the user sees on every commit. The original ASYNC-6 fix (TASK-0589) chose this implementation explicitly for simplicity; revisiting now that the pattern is recognised as a code-review-rust regression site.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 child wait blocks until either exit or deadline without periodic sleeps (use wait-timeout crate or platform pidfd/kqueue primitive)
- [x] #2 happy-path latency for a fast git diff --cached drops below the current 50ms floor
- [x] #3 timeout-on-hang test still passes within current 5s upper bound
<!-- AC:END -->
