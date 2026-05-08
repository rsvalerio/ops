---
id: TASK-1092
title: >-
  ERR-1: resolve_env_usize conflates Ok(0) with Err(_), silently rejecting valid
  'sequential' intent
status: Done
assignee: []
created_date: '2026-05-07 21:32'
updated_date: '2026-05-08 06:37'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/parallel.rs:117-142`

**What**: The `Ok(0) | Err(_)` arm conflates "user passed 0 to disable parallelism" with "garbage value", silently rejecting a valid intent of "single-threaded". An explicitly-set empty string `OPS_MAX_PARALLEL=` returns `Ok("")` which fails parse and warns with `value = ""` — ambiguous diagnostic.

**Why it matters**: Operators trying to force single-threaded execution get a silent override; the warn message doesn't explain why their setting was rejected.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The 0 case warns with a distinct message ('zero is not allowed; use 1 for sequential') so users understand why their setting was overridden
- [x] #2 An empty-string input is detected explicitly and flagged separately, or silently treated as unset
- [x] #3 A unit test pins the warn-message contents for the zero case
<!-- AC:END -->
