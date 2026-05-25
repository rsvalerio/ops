---
id: TASK-1069
title: >-
  PATTERN-1: about workspace resolve_member_globs uses find('*') and silently
  flattens prefix/*/suffix and **/foo
status: Done
assignee: []
created_date: '2026-05-07 21:18'
updated_date: '2026-05-08 06:36'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:31-33`

**What**: `resolve_member_globs` finds the first `*` and treats everything before it as a literal prefix, ignoring trailing segments. A bare `**/foo` has prefix `""`, so the helper enumerates the entire workspace root and tries to load `marker` from every top-level dir. Multi-segment globs that legitimately match deep paths silently miss them.

**Why it matters**: Users authoring `.ops.toml` or upstream workspace manifests with `**/foo` or `prefix/*/suffix` get either a brute-force scan of the entire root (`**/foo` case) or silently dropped patterns — neither matches their intent and there is no warning.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Skip patterns whose suffix is non-empty (or implement proper recursive glob), rather than wildcard-matching only the prefix
- [x] #2 Emit tracing::warn! once per non-suffix-trivial pattern so the divergence is observable
- [x] #3 Document the supported glob shapes in user-facing config docs
<!-- AC:END -->
