---
id: TASK-1548
title: >-
  READ-5: types.rs Metadata constructors duplicate OnceLock::new() initialiser
  block across from_value and from_context
status: To Do
assignee:
  - TASK-1576
created_date: '2026-05-19 15:26'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - READ
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:123-144`

**What**: `Metadata::from_value` (lines 123-131) and `Metadata::from_context` (lines 135-144) each construct a `Self { inner, member_ids: OnceLock::new(), default_member_ids: OnceLock::new(), package_index_by_name: OnceLock::new(), package_index_by_id: OnceLock::new() }` literal. If a new lazy cache field is added to `Metadata` (e.g. `targets_by_kind: OnceLock<...>`), both constructors must be edited in lockstep.

**Why it matters**: This is a small READ-5 / DUP-1 hybrid — two constructors restating the same four-field initialiser is a maintenance trap whenever the cache surface grows. Either `impl Default for Metadata` (so constructors do `Self { inner, ..Default::default() }`), or extract a private `fn empty_caches() -> (OnceLock<_>, OnceLock<_>, OnceLock<_>, OnceLock<_>)`, or — best — derive `Default` on a `MetadataCaches` substruct and embed it. The current shape works but encodes the same cache list in two places.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Adding a new lazy cache field to Metadata requires editing exactly one initialiser, not two
- [ ] #2 from_value and from_context bodies do not enumerate the OnceLock fields by hand
<!-- AC:END -->
