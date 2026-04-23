---
id: TASK-0235
title: >-
  API-9: ExecCommandSpec lacks #[non_exhaustive] despite public extension-facing
  API
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
**File**: `crates/core/src/config/mod.rs:287`

**What**: Public struct exposes all fields; adding a field is a breaking change for extension crates constructing ExecCommandSpec literals.

**Why it matters**: Extensions pattern-match or construct literals, so future field additions silently break downstream users.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Apply #[non_exhaustive] to ExecCommandSpec
- [ ] #2 Document builder or constructor path
<!-- AC:END -->
