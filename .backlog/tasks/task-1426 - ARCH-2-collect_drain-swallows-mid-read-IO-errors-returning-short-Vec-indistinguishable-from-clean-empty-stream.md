---
id: TASK-1426
title: >-
  ARCH-2: collect_drain swallows mid-read IO errors, returning short Vec
  indistinguishable from clean empty stream
status: To Do
assignee:
  - TASK-1457
created_date: '2026-05-13 18:22'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - ARCH
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:449`

**What**: When `read_capped` returns `(buf, dropped, Some(err))`, `collect_drain` emits a `tracing::warn!` and returns `Ok(buf)`. Callers cannot distinguish a clean empty stream from a partially-read-then-failed one.

**Why it matters**: This contradicts the panic-handling contract documented around subprocess.rs:309-322 ("an empty value here always means the child produced no output, never that we lost it"). A mid-read EIO produces a short or empty Vec that callers treat as authoritative.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 When read_capped returns Some(err) and buf is empty, surface RunError::Io instead of Ok
- [ ] #2 Document partial-read semantics explicitly (or attach a partial-flag to Output)
- [ ] #3 Regression test pinning the error path
<!-- AC:END -->
