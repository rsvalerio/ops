---
id: TASK-0304
title: 'ASYNC-6: OPS_SUBPROCESS_TIMEOUT_SECS env override has no upper bound'
status: Done
assignee:
  - TASK-0323
created_date: '2026-04-24 08:52'
updated_date: '2026-04-25 12:29'
labels:
  - rust-code-review
  - async
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:83-89`

**What**: `run_with_timeout` reads `OPS_SUBPROCESS_TIMEOUT_SECS` and accepts any non-zero u64. A very large value (e.g. `u64::MAX`) effectively disables the timeout.

**Why it matters**: The whole point of the helper is bounded subprocess execution. An env-driven effective disable undermines the ASYNC-6 contract for every caller that migrated to this helper.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Env value clamped to a sane maximum (e.g. 3600s) with a tracing::warn when clamped
- [x] #2 Unit test added covering clamp behavior and zero-value rejection
<!-- AC:END -->
