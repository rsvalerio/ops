---
id: TASK-0372
title: 'CONC-5: Polling busy-loop with thread::sleep for subprocess timeout'
status: To Do
assignee:
  - TASK-0421
created_date: '2026-04-26 09:37'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - concurrency
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/timeout.rs:14`

**What**: run_with_timeout polls try_wait every 200ms in a hot loop with std::thread::sleep. This wakes the thread 5 times/sec for the entire install duration (up to 10 minutes), wasting CPU and adds up to 200ms of latency.

**Why it matters**: Inefficient and imprecise. Better: platform-specific blocking wait with a timeout (e.g., wait_timeout crate) or signal-driven waitpid.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace polling loop with a blocking primitive that sleeps until process exits or deadline elapses
- [ ] #2 Tests still pass; timeout tests verify both timeout-fire and fast-exit behavior
<!-- AC:END -->
