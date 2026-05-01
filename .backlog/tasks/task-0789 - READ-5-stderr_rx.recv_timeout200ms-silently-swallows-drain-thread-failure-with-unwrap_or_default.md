---
id: TASK-0789
title: >-
  READ-5: stderr_rx.recv_timeout(200ms) silently swallows drain-thread failure
  with unwrap_or_default
status: Triage
assignee: []
created_date: '2026-05-01 05:58'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/run-before-commit/src/lib.rs:181`

**What**: After child exit, the stderr drain thread's Vec<u8> is read via recv_timeout(Duration::from_millis(200)).unwrap_or_default(). The fixed 200 ms cap is reasonable, but unwrap_or_default() swallows both the timeout case and a genuine RecvError (sender dropped before sending).

**Why it matters**: ERR-1 — handle or propagate, never both. In the error branch the user's stderr message can be a misleading empty string when the drain thread crashed (e.g. allocator failure on huge stderr).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Distinguish RecvTimeoutError::Timeout vs Disconnected; emit a tracing::debug! for the disconnected case so operators have a breadcrumb
- [ ] #2 Test exercising the disconnected branch (drop the sender) confirms the warn path
<!-- AC:END -->
