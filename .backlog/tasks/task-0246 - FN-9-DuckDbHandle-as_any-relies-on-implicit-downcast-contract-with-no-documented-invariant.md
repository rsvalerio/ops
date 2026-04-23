---
id: TASK-0246
title: >-
  FN-9: DuckDbHandle::as_any relies on implicit downcast contract with no
  documented invariant
status: To Do
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 06:46'
labels:
  - rust-code-review
  - function-design
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:141`

**What**: Trait exposes as_any allowing arbitrary downcast but the safety/type contract is not documented at the trait level.

**Why it matters**: Callers must read implementations to know what concrete type is safe to downcast.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Document the expected concrete type(s)
- [ ] #2 Provide a typed accessor method instead of raw as_any
<!-- AC:END -->
