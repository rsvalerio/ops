---
id: TASK-0542
title: 'PERF-3: output_byte_cap re-reads OPS_OUTPUT_BYTE_CAP env per command spawn'
status: Done
assignee:
  - TASK-0643
created_date: '2026-04-29 04:58'
updated_date: '2026-04-29 14:23'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:95`

**What**: `CommandOutput::from_raw` invokes `output_byte_cap()` which calls `std::env::var(OUTPUT_CAP_ENV)` on every spawn. A 100-step parallel plan performs 100 env-var lookups; under MAX_PARALLEL=32 several contend on the process-global env lock.

**Why it matters**: The cap is process-global and constant for a run; resolving once at startup or first call removes the per-spawn overhead.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 output_byte_cap is memoized via OnceLock<usize> (or read once at CommandRunner construction)
- [x] #2 Existing override / fallback semantics are preserved with a regression test that verifies the value sticks across many from_raw calls
<!-- AC:END -->
