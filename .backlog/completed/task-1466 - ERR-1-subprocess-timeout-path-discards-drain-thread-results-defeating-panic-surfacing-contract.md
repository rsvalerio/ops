---
id: TASK-1466
title: >-
  ERR-1: subprocess timeout path discards drain-thread results, defeating
  panic-surfacing contract
status: Done
assignee:
  - TASK-1479
created_date: '2026-05-15 18:51'
updated_date: '2026-05-17 07:31'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:409-412` (approximate — the timeout branch of `run_with_timeout_inner`)

**What**: When the deadline expires, both drain joins are swallowed via `let _ = collect_drain(...)`. `collect_drain` was hardened (ARCH-2 / ERR-1) so a panicking drain thread surfaces as `RunError::Io`. The timeout branch defeats that contract: a drain-thread panic that coincides with a timeout becomes invisible — operators see only `Timeout` with no breadcrumb that the captured bytes were unrecoverable.

**Why it matters**: Diagnostic regressions are exactly what the prior ERR-1 hardening of `collect_drain` was meant to prevent. The discarded-result path is the only place the contract is broken.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Timeout branch surfaces a tracing::warn! when a drain join errors or panics, naming label and stream (stdout/stderr)
- [ ] #2 Regression test injects a panicking drain reader plus a timeout and asserts the warn breadcrumb fires alongside RunError::Timeout
<!-- AC:END -->
