---
id: TASK-1591
title: 'TEST-19: timeout.rs tests invoke `sh` without #[cfg(unix)] gate'
status: Done
assignee:
  - TASK-1638
created_date: '2026-05-21 22:49'
updated_date: '2026-05-22 13:17'
labels:
  - code-review-rust
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/timeout.rs:35-65`

**What**: The `tests` module in `src/timeout.rs` is gated only by `#[cfg(test)]`. Both tests (`run_with_timeout_fires_on_hung_subprocess` and `run_with_timeout_succeeds_for_fast_subprocess`) spawn `Command::new("sh")` and `.expect("sh must be available")`. On Windows hosts (no `sh` on PATH by default), both tests will fail with a spawn error, not a missed assertion.

For comparison, the sibling probe-side timeout test in `extensions-rust/tools/src/probe/timeout.rs:121` is properly gated `#[cfg(all(test, unix))]` for the same reason.

**Why it matters**: TEST-19 (platform portability) — making `cargo test -p ops-tools` non-portable to Windows for a reason that is not the code under test. The behaviour being verified (`wait_timeout`-based bounded wait) is itself cross-platform; only the test harness needs `sh`. The fix is a one-line cfg tightening so the suite degrades cleanly on Windows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 tests module in extensions-rust/tools/src/timeout.rs is gated #[cfg(all(test, unix))] (matching probe/timeout.rs)
- [x] #2 cargo test -p ops-tools does not attempt to spawn sh on non-unix targets
<!-- AC:END -->
