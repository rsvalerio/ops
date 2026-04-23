---
id: TASK-0275
title: 'TEST-5: resolve_theme has no direct unit tests — NotFound branch uncovered'
status: Done
assignee: []
created_date: '2026-04-23 06:37'
updated_date: '2026-04-23 15:23'
labels:
  - rust-code-review
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/lib.rs:32`

**What**: Only exercised indirectly through ConfigurableTheme render tests.

**Why it matters**: Regression to resolver logic would not be caught.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add resolve_theme_returns_not_found and resolve_theme_hits tests
- [ ] #2 Cover list_theme_names ordering
<!-- AC:END -->
