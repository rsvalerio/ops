---
id: TASK-1549
title: >-
  API-1: MetadataExtension lacks must_use and Debug despite being part of the
  crate's public factory surface
status: Done
assignee:
  - TASK-1576
created_date: '2026-05-19 15:26'
updated_date: '2026-05-19 17:48'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/lib.rs:103-104`

**What**: `pub struct MetadataExtension;` is declared `#[non_exhaustive]` but has no `#[must_use]`, no `Debug` derivation, and no documented invariant beyond the one-liner "construct via the registered extension factory only". The `Metadata` types it indirectly produces are public, but `MetadataExtension` itself is also `pub` (it's the type the factory `METADATA_FACTORY` returns, see `impl_extension!` macro at line 106-120).

**Why it matters**: API-1 covers public types missing the standard derives. `MetadataExtension` is a zero-field unit struct whose `Debug` is trivial; without it, downstream code cannot include it in `tracing::debug!(?ext)` or `assert_eq!` calls. The `#[must_use]` is debatable for a unit struct, but the missing `Debug` derive is the easy half of API-1.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 MetadataExtension derives Debug
- [ ] #2 Public re-exports of MetadataExtension (if any) compose Debug as expected
<!-- AC:END -->
