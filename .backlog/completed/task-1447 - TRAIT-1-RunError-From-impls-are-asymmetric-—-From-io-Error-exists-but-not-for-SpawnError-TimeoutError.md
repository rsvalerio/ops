---
id: TASK-1447
title: >-
  TRAIT-1: RunError From impls are asymmetric — From<io::Error> exists but not
  for SpawnError/TimeoutError
status: Done
assignee:
  - TASK-1456
created_date: '2026-05-13 18:45'
updated_date: '2026-05-14 07:42'
labels:
  - code-review-rust
  - traits
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/subprocess.rs:169-173`

**What**: `RunError::from(io::Error)` is provided, but `SpawnError` and `TimeoutError` must be constructed via the variant constructor explicitly. Readers see `From<io::Error>` and assume the `?` propagation path works for all variants.

**Why it matters**: Minor ergonomics, but the asymmetry is a foot-gun. Either remove the partial `From` (force constructor everywhere) or add the missing two impls so `?` works uniformly. Consider switching to `thiserror`-derived `From` impls for consistency.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either remove From<io::Error> for RunError OR add From<SpawnError> and From<TimeoutError>
- [ ] #2 Caller sites that construct RunError::Spawn / RunError::Timeout manually compile unchanged or are migrated
- [ ] #3 Decision is documented in a one-line doc comment on RunError
<!-- AC:END -->
