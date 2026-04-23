---
id: TASK-0274
title: 'API-9: Public ThemeError enum is not #[non_exhaustive]'
status: Done
assignee: []
created_date: '2026-04-23 06:37'
updated_date: '2026-04-23 15:22'
labels:
  - rust-code-review
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/lib.rs:24`

**What**: Adding NotConfigured/InvalidField variants later is breaking.

**Why it matters**: Theme crate is extension-facing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Annotate ThemeError with #[non_exhaustive]
- [ ] #2 Document variant semantics
<!-- AC:END -->
