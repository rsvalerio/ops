---
id: TASK-0359
title: >-
  TEST-11: data_provider_error_is_clone asserts only string equality, not Arc
  reuse or chain preservation
status: Done
assignee:
  - TASK-0421
created_date: '2026-04-26 09:36'
updated_date: '2026-04-27 16:03'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/tests.rs:413`

**What**: data_provider_error_is_clone asserts err.to_string() == cloned.to_string() after cloning a DataProviderError::ComputationFailed. Does not verify variant survived the clone, that the underlying Arc was reused (point of EFF-002), or that source() chain is preserved.

**Why it matters**: A regression replacing Clone with a to_string-then-rewrap implementation would still pass while breaking the documented EFF-002 invariant.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add Arc::ptr_eq (or equivalent) check between the inner Arc of the original and the clone
- [x] #2 Add matches!(cloned, DataProviderError::ComputationFailed(_)) and cloned.source().is_some() assertions
<!-- AC:END -->
