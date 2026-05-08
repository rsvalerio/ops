---
id: TASK-1227
title: >-
  TRAIT-9: DuckDbHandle trait erasure relies on each implementer correctly
  returning self from as_any
status: To Do
assignee:
  - TASK-1269
created_date: '2026-05-08 12:57'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - traits
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:281-287`

**What**: The `DuckDbHandle` trait bounds Any access through `fn as_any(&self) -> &dyn std::any::Any` with a doc-only contract that "the implementer must return self". A buggy or hostile implementer returning `&()` would silently break every downcast — `get_db` returns None and no DB-backed feature works.

**Why it matters**: The contract is documentation-enforced rather than type-enforced; a future implementer (test mock) could violate it without compile-time detection.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Provide a default impl via a sealed extension trait pattern
- [ ] #2 OR delete as_any in favour of an enum/typed handle since the doc admits one concrete type
- [ ] #3 Existing downcast call sites unchanged
<!-- AC:END -->
