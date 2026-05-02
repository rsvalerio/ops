---
id: TASK-0901
title: >-
  ERR-1: collect_drain replaces panicked drain thread output with empty Vec,
  success looks output-less
status: Triage
assignee: []
created_date: '2026-05-02 10:08'
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
- [ ] #1 Drain-thread panic is propagated as RunError::Io rather than Vec::new() with only a tracing breadcrumb
- [ ] #2 Update doc on run_with_timeout to enumerate panic-handling guarantees
<!-- AC:END -->
