---
id: TASK-0519
title: 'READ-2: validate_cargo_tool_arg loop redundantly re-checks the first character'
status: To Do
assignee:
  - TASK-0533
created_date: '2026-04-28 06:52'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/tools/src/install.rs:32`

**What**: After validating that `first` is alphanumeric, the loop is `for ch in std::iter::once(first).chain(chars)` — so the alphanumeric check on `first` is repeated against the broader allowed set. The leading-dash gate works only because the explicit alphanumeric check rejects `-` first.

**Why it matters**: A future reader who deletes the explicit alphanumeric check (thinking the loop covers it) silently re-introduces leading-dash acceptance, defeating SEC-13.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Loop iterates only the remaining chars after the first
- [ ] #2 Existing tests for leading-dash, embedded special chars, and empty input still pass
<!-- AC:END -->
