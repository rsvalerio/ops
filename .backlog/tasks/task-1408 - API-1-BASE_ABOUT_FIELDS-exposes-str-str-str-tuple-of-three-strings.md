---
id: TASK-1408
title: 'API-1: BASE_ABOUT_FIELDS exposes &[(&str, &str, &str)] tuple-of-three-strings'
status: Done
assignee:
  - TASK-1452
created_date: '2026-05-13 18:10'
updated_date: '2026-05-13 20:35'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity.rs:310-327`

**What**: Public `BASE_ABOUT_FIELDS: &[(&str, &str, &str)]` carries positional `(id, label, description)` fields. Downstream consumers must know slot order; the existing `AboutFieldDef` struct already names these fields but is bypassed at the public surface.

**Why it matters**: Exposing the const as `&[AboutFieldDef]` (now possible with const constructors), or keeping only the `base_about_fields()` accessor public, removes the positional-tuple footgun without changing the data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Public surface no longer exposes a positional 3-tuple of strings for about-field metadata
<!-- AC:END -->
