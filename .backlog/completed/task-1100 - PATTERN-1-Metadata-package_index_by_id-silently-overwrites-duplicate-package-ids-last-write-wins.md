---
id: TASK-1100
title: >-
  PATTERN-1: Metadata::package_index_by_id silently overwrites duplicate package
  ids (last-write-wins)
status: Done
assignee: []
created_date: '2026-05-07 21:33'
updated_date: '2026-05-08 04:18'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/types.rs:162-176`

**What**: Sister of TASK-1019 (`package_index_by_name`). The id index uses `.collect()` into a HashMap, so a duplicate `id` value from `cargo metadata` (path-dep aliases, vendored crate same-id collision, future schema where ids reuse) silently keeps the last entry with no warn. Callers of `package_by_id` see one package while iteration would have shown two.

**Why it matters**: Cargo metadata IDs are *usually* unique but the contract is not enforced here, identical shape to TASK-1019.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Duplicate ids in inner["packages"] emit a single tracing::warn! per collision while building the index
- [ ] #2 Behaviour is documented (last-write-wins explicit, not accidental) in the doc comment
- [ ] #3 Unit test pins the warn fires on a synthetic two-entry duplicate-id metadata blob
<!-- AC:END -->
