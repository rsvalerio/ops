---
id: TASK-0870
title: >-
  TRAIT-4: extensions-rust UpdateAction derives Clone but not Copy despite being
  unit-variant enum
status: Done
assignee: []
created_date: '2026-05-02 09:22'
updated_date: '2026-05-02 10:49'
labels:
  - code-review-rust
  - traits
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-update/src/lib.rs:25-32`

**What**: UpdateAction is a unit-variant enum (Update, Add, Remove). It derives Clone but not Copy, forcing call sites to write action.clone() (line 240, 254). Copy is semantically appropriate and a non-breaking addition.

**Why it matters**: Calls in the hot parsing loop call .clone() on a 1-byte enum, which is style-noise. Adding Copy removes the noise and matches the convention used elsewhere (VersionRole already derives Copy).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 derive(Copy) added to UpdateAction
- [ ] #2 .clone() calls on UpdateAction removed
- [ ] #3 No public-API regressions (Copy is purely additive)
<!-- AC:END -->
