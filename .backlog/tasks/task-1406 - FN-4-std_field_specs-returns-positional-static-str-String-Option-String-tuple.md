---
id: TASK-1406
title: >-
  FN-4: std_field_specs returns positional (&'static str, String,
  Option<String>) tuple
status: Done
assignee:
  - TASK-1452
created_date: '2026-05-13 18:10'
updated_date: '2026-05-13 20:35'
labels:
  - code-review-rust
  - FN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:22-65`

**What**: `std_field_specs` returns `Vec<(&'static str, String, Option<String>)>`. Callers destructure positionally (see card.rs:121-122), so slot order must be remembered at every site.

**Why it matters**: A named `FieldSpec { id, label, value }` struct would make filters like `filter(|(fid, ..)| show(fid))` self-documenting and survive future field additions without silently shifting the meaning of slot 0/1/2 at every call site.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 std_field_specs returns a named struct rather than a 3-tuple
<!-- AC:END -->
