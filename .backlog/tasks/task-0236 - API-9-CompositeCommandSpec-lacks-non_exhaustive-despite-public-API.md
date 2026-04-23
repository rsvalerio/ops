---
id: TASK-0236
title: 'API-9: CompositeCommandSpec lacks #[non_exhaustive] despite public API'
status: Done
assignee: []
created_date: '2026-04-23 06:34'
updated_date: '2026-04-23 14:29'
labels:
  - rust-code-review
  - api
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/config/mod.rs:354`

**What**: Struct is publicly constructible; adding fail-fast-style fields later is a breaking change.

**Why it matters**: Hinders adding composite options without a major version bump.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Apply #[non_exhaustive]
- [ ] #2 Add constructor/builder for external construction
<!-- AC:END -->
