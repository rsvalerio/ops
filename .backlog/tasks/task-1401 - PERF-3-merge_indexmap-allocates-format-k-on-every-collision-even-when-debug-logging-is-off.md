---
id: TASK-1401
title: >-
  PERF-3: merge_indexmap allocates format!("{k:?}") on every collision even when
  debug logging is off
status: To Do
assignee:
  - TASK-1453
created_date: '2026-05-13 18:09'
updated_date: '2026-05-13 19:09'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/merge.rs:33`

**What**: Each collision in `merge_indexmap` pays a `format!("{k:?}")` allocation purely to feed `tracing::debug!`. The macro's own `tracing::enabled!` short-circuit is bypassed because the formatted string is constructed before the macro receives it.

**Why it matters**: When `OPS_LOG_LEVEL` is not debug, the allocation is wasted on every collision during overlay merge. Either pass `?k` directly as a structured field (lazy) or gate with `tracing::event_enabled!(Level::DEBUG)`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Format allocation does not run when debug tracing is disabled
<!-- AC:END -->
