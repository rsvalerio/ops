---
id: TASK-0901
title: >-
  ERR-1: collect_drain replaces panicked drain thread output with empty Vec,
  success looks output-less
status: Done
assignee: []
created_date: '2026-05-02 10:08'
updated_date: '2026-05-02 14:49'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:251`

**What**: If a stdout/stderr drain thread panics, collect_drain logs at warn but returns Vec::new(). run_with_timeout then returns Ok(Output) with empty stdout/stderr and the original exit status, so a successful command appears to have produced no output — indistinguishable from a clean empty stream.

**Why it matters**: Cargo callers parse stdout to make decisions (cargo metadata, cargo update); a silently-empty buffer can drive downstream logic to wrong conclusions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Drain-thread panic is propagated as RunError::Io rather than Vec::new() with only a tracing breadcrumb
- [x] #2 Update doc on run_with_timeout to enumerate panic-handling guarantees
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
collect_drain returns Result<Vec<u8>, RunError>; a panicked drain thread now propagates as RunError::Io instead of an empty Vec. run_with_timeout uses ? on each drain, so Output.stdout/stderr are guaranteed to be "exactly what the child wrote" (empty means empty, never "we lost it"). Doc on run_with_timeout enumerates the panic-handling guarantees explicitly.
<!-- SECTION:NOTES:END -->
