---
id: TASK-1584
title: >-
  ERR-9: timeout.rs run_with_timeout silently drops kill+wait errors on timeout
  path
status: Done
assignee:
  - TASK-1638
created_date: '2026-05-21 22:45'
updated_date: '2026-05-22 13:15'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/timeout.rs:26-31`

**What**: When `wait_timeout` returns `None`, the code does `let _ = child.kill();` followed by `let _ = child.wait();` before bailing. Both errors are silently discarded with no log. A kill() failure (e.g. process already in a kernel-uninterruptible state, missing capability) means the bailed error tells the operator "timed out" but leaves a zombie/runaway child that we wrongly believe was reaped.

**Why it matters**: ERR-9 — discarded errors hide reliability problems. The bail correctly surfaces the deadline, but a failing kill is the higher-severity event (operator now has an orphaned cargo/rustup install eating disk I/O until the kernel notices) and deserves at minimum a `tracing::warn!` so post-incident logs explain why the next install attempt sees `already installing` or filesystem lock contention.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 kill() failure on the timeout path is logged via tracing::warn with label/timeout context
- [x] #2 wait() failure on the timeout path is logged via tracing::warn (or explicitly justified in a comment)
- [x] #3 existing run_with_timeout_fires_on_hung_subprocess test still passes
<!-- AC:END -->
