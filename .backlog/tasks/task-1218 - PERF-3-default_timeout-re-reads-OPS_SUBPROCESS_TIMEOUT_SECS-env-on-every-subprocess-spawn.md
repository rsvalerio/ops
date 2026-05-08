---
id: TASK-1218
title: >-
  PERF-3: default_timeout re-reads OPS_SUBPROCESS_TIMEOUT_SECS env on every
  subprocess spawn
status: To Do
assignee:
  - TASK-1262
created_date: '2026-05-08 12:56'
updated_date: '2026-05-08 13:18'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:247-265`

**What**: `default_timeout` calls `std::env::var(TIMEOUT_ENV)` on every subprocess spawn. The env var is process-global and constant, but the sibling `output_byte_cap` (line 180) caches its env knob behind a `OnceLock` while this function does not.

**Why it matters**: Under parallel runner spawns, every call pays a global env-lock acquisition and `String` allocation. Inconsistent with the OnceLock pattern used one function above for an identical knob shape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Resolve TIMEOUT_ENV once via OnceLock<Option<u64>>
- [ ] #2 Apply MAX_TIMEOUT_SECS clamp at cache init so the warn fires once
- [ ] #3 Existing default_timeout tests pass via accept-snapshot semantics or test reset hook
<!-- AC:END -->
