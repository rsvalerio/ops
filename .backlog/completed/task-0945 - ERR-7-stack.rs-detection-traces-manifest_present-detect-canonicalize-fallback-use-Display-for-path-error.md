---
id: TASK-0945
title: >-
  ERR-7: stack.rs detection traces (manifest_present, detect canonicalize
  fallback) use Display for path/error
status: Done
assignee: []
created_date: '2026-05-02 16:03'
updated_date: '2026-05-02 17:26'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/stack.rs:23-26, 167-170` (and any sibling site at ~248-249)

**What**: Multiple `tracing::debug!` events inside stack-detection (manifest_present probe, canonicalize fallback in Stack::detect) log `path = %path.display(), error = %err` using Display. Paths come from CWD-relative ancestor traversal so are attacker-controllable.

**Why it matters**: TASK-0818 sweep policy is Debug formatter for all path/error tracing fields to defeat log-injection. These sites were missed. Sister to TASK-0930/0937.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Switch path and error fields to Debug formatter at every tracing event in stack.rs (verify by grep that no Display %path.display() or %err remains)
- [x] #2 Regression test asserts escaping of embedded \n/\u{1b} in a stack-detection path
<!-- AC:END -->
