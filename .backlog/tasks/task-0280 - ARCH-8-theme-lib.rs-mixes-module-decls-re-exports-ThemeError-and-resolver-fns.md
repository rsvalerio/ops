---
id: TASK-0280
title: >-
  ARCH-8: theme/lib.rs mixes module decls, re-exports, ThemeError, and resolver
  fns
status: Done
assignee: []
created_date: '2026-04-23 06:37'
updated_date: '2026-04-23 15:26'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/theme/src/lib.rs:1`

**What**: Rule says lib.rs should be a thin entry point.

**Why it matters**: Mild today but will drift as ThemeError grows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Extract ThemeError to error.rs
- [ ] #2 Move resolve_theme/list_theme_names to resolver.rs
<!-- AC:END -->
