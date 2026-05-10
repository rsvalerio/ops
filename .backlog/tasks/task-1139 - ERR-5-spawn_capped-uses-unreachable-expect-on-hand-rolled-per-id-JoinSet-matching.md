---
id: TASK-1139
title: >-
  ERR-5: spawn_capped uses unreachable!/expect on hand-rolled per-id JoinSet
  matching
status: Done
assignee:
  - TASK-1268
created_date: '2026-05-08 07:41'
updated_date: '2026-05-09 17:30'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/exec.rs:140-162`

**What**: The hand-rolled per-id matching loop relies on `unreachable!(\"only stdout/stderr drains spawned\")` and `expect(\"stdout drain awaited\")` to encode \"exactly two tasks.\" Structurally true today, but a refactor adding any third task to `drains` (e.g. drain stdin, watchdog) trips a panic inside the runtime worker — and the panic payload (file path, format args) propagates via `JoinError` to `collect_join_results`. SEC-21/TASK-0334's redaction explicitly only covers the *outer* parallel JoinSet, not this inner one.

**Why it matters**: The invariant is structural and currently safe but the failure mode if it breaks is a panic-with-payload-in-process listing leaking into CI logs. Same pattern was hardened for the outer JoinSet (TASK-0334).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Replace unreachable! arms with a logged-and-returned io::Error
- [x] #2 Drop the two expects in favour of ?/ok_or_else so a regression surfaces as StepFailed not a panic
<!-- AC:END -->
