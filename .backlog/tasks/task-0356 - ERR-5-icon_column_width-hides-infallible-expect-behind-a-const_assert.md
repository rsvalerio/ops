---
id: TASK-0356
title: 'ERR-5: icon_column_width hides infallible expect behind a const_assert'
status: To Do
assignee:
  - TASK-0418
created_date: '2026-04-26 09:35'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/step_line_theme.rs:210`

**What**: icon_column_width uses .max().expect("ALL_STATUSES is guaranteed non-empty by const assert") after a sibling const _: () = assert!(!ALL_STATUSES.is_empty(), ...). The const-assert is in the trait default body and is re-instantiated per impl.

**Why it matters**: A panic-free fold or .unwrap_or(0) removes the panic path entirely and the doc/const-assert pair, simplifying without losing safety.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace .max().expect(...) and adjacent const _ : () = assert!(...) with a panic-free fold or .unwrap_or(0)
- [ ] #2 Existing tests still pass; remove the now-dead assert! and accompanying comment
<!-- AC:END -->
