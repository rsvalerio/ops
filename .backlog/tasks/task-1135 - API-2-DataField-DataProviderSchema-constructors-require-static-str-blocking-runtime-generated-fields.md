---
id: TASK-1135
title: >-
  API-2: DataField/DataProviderSchema constructors require &'static str blocking
  runtime-generated fields
status: To Do
assignee:
  - TASK-1269
created_date: '2026-05-08 07:40'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:26`

**What**: `DataField::new`, `DataProviderSchema::new`, and related fields require `&'static str`. Convenient for `data_field!` macro but forces extensions wanting programmatic field descriptions to leak strings via `Box::leak(...)` or unsafe `'static` casts.

**Why it matters**: For an extension framework type marked `#[non_exhaustive]` and handed to third-party crates, the `'static` constraint is under-documented and forces unsafe leak patterns when fields are computed at runtime.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either document the 'static constraint with rationale, or migrate to Cow<'static, str>
- [ ] #2 Verify data_field! macro still works against the new shape
<!-- AC:END -->
