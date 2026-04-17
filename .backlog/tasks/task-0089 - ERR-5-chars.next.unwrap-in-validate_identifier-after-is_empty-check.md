---
id: TASK-0089
title: 'ERR-5: chars.next().unwrap() in validate_identifier after is_empty check'
status: Done
assignee: []
created_date: '2026-04-17 11:33'
updated_date: '2026-04-17 14:56'
labels:
  - rust-codereview
  - err
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/validation.rs:26`

**What**: After name.is_empty() check, chars.next().unwrap() is used to grab the first char; correct today but uses unwrap() in production code.

**Why it matters**: Any refactor that moves the emptiness check risks panic; prefer infallible patterns.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Replace unwrap with let Some(first) = chars.next() else { return Err(...) }; collapsing the two guards
- [ ] #2 Add test for empty-name rejection to ensure the refactor preserves behavior
<!-- AC:END -->
