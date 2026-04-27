---
id: TASK-0398
title: 'ARCH-9: PomData fields are pub despite never being used outside maven.rs'
status: To Do
assignee:
  - TASK-0417
created_date: '2026-04-26 09:41'
updated_date: '2026-04-26 10:10'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/maven.rs:47`

**What**: pub(crate) struct PomData { pub artifact_id: ..., ... } exposes every field as pub, but PomData is pub(crate) and is constructed/read only inside this module.

**Why it matters**: ARCH-9 — minimal public surface. Public fields lock the struct shape into the crate API and prevent later refactoring.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Drop pub on each field and provide a constructor or Default if internal callers need one; keep pub(crate) on the struct itself only if it must cross modules
- [ ] #2 If PomData is module-private (no use from lib.rs or gradle.rs), demote to plain struct PomData (no pub(crate))
<!-- AC:END -->
