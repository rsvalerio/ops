---
id: TASK-1611
title: >-
  PATTERN-3: test-coverage flatten_coverage_json uses Vec<&Vec<Value>> where
  Vec<&[Value]> suffices
status: Done
assignee:
  - TASK-1634
created_date: '2026-05-22 06:49'
updated_date: '2026-05-22 08:53'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/parse.rs:201`

**What**: `let file_arrays: Vec<&Vec<serde_json::Value>> = ...` borrows `Vec`s where the only consumers are `.iter()` and `.len()`, both of which are satisfied by `&[T]`.

**Why it matters**: `&Vec<T>` is the canonical clippy::ptr_arg smell — it pins the source type to `Vec`, blocks slice/array sources, and adds an extra indirection for no benefit. Trivial fix; removes incidental coupling.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The local type is Vec<&[serde_json::Value]> or the binding is elided into an iterator chain
- [ ] #2 flatten_coverage_json semantics and all existing tests pass unchanged
<!-- AC:END -->
