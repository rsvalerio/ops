---
id: TASK-1420
title: >-
  PERF-3: push_special_fields scans visible_fields linearly per field via
  repeated .iter().any()
status: Done
assignee:
  - TASK-1452
created_date: '2026-05-13 18:18'
updated_date: '2026-05-13 20:35'
labels:
  - code-review-rust
  - perf
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity/card.rs:67`

**What**: The `show` closure built in `from_identity_filtered` (line 112-117) and passed into both `std_field_specs` filter and `push_special_fields` does `visible_fields.iter().any(|f| f == field_id)` for every field check, including `authors` and `coverage`. With nine std fields plus two special fields, this is O(N*M). Same shape was already collapsed in `about_cmd::defaults` (TASK-1332).

**Why it matters**: Symmetric to the already-closed TASK-1332. Build a `HashSet<&str>` (or sorted slice with binary_search) once at the top of `from_identity_filtered` and check membership in O(1). About-card rendering is on the default `ops` invocation path.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 build HashSet<&str> from visible_fields once and use O(1) contains() in the show closure
- [ ] #2 preserve existing all-fields-when-None behaviour
- [ ] #3 regression test pins identical filtered output for a representative visible_fields config
<!-- AC:END -->
