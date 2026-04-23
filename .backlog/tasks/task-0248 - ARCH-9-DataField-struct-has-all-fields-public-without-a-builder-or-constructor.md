---
id: TASK-0248
title: >-
  ARCH-9: DataField struct has all fields public without a builder or
  constructor
status: Done
assignee: []
created_date: '2026-04-23 06:35'
updated_date: '2026-04-23 09:02'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/data.rs:10`

**What**: Public struct with three public fields means additions are breaking changes for extension consumers using struct literals.

**Why it matters**: Extension-facing API — same class of breakage as API-9 for commands.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Apply #[non_exhaustive] or provide DataField::new(...) + private fields
- [x] #2 Update data_field! macro to use constructor
<!-- AC:END -->
