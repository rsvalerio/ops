---
id: TASK-1065
title: >-
  PERF-3: insert_thousands_separators allocates twice and double-reverses on
  every call
status: Done
assignee: []
created_date: '2026-05-07 21:18'
updated_date: '2026-05-08 06:29'
labels:
  - code-review-rust
  - PERF
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/text.rs:74-83`

**What**: Builds `result` via reversed iteration into a `String`, then does `result.chars().rev().collect()` to flip back. Every call to `format_number` runs over UTF-8 char iteration twice; called per About-card / table render.

**Why it matters**: Hot in render paths (about cards, identity metrics). A forward implementation that computes leading-group length is straightforward and zero-alloc for the common `n.abs() < 1000` case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Single forward pass; no second allocation or chars().rev() round-trip
- [x] #2 Zero-alloc fast path for n.abs() < 1000
- [x] #3 Existing tests for i64::MIN / MAX / negative / zero still pass
<!-- AC:END -->
