---
id: TASK-0515
title: >-
  PERF-1: CommandOutput::from_raw stores entire stdout/stderr as String with no
  cap
status: Done
assignee:
  - TASK-0537
created_date: '2026-04-28 06:51'
updated_date: '2026-04-28 16:54'
labels:
  - code-review-rust
  - performance
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/runner/src/command/results.rs:87`

**What**: String::from_utf8_lossy(...).into_owned() copies the full child output into a String per StepResult, even when only the stderr tail is rendered to the user.

**Why it matters**: Documented under EFF-004 but no cap is enforced; a pathological build emitting hundreds of MB of output balloons memory per step. A configurable byte cap (or streaming tail buffer) would bound the worst case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Cap captured stdout/stderr at a configurable byte limit
- [x] #2 Drop overflow with a marker line at the tail
<!-- AC:END -->
