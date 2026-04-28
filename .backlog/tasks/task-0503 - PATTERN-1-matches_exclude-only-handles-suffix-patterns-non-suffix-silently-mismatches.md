---
id: TASK-0503
title: >-
  PATTERN-1: matches_exclude only handles suffix-* patterns; non-suffix *
  silently mismatches
status: To Do
assignee:
  - TASK-0534
created_date: '2026-04-28 06:50'
updated_date: '2026-04-28 07:14'
labels:
  - code-review-rust
  - correctness
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/about/src/workspace.rs:61`

**What**: matches_exclude returns false whenever `*` is not the last char of the pattern (e.g. `packages/internal-*-tool`, `pkg/*/foo`). The same module's resolve_member_globs happily expands prefix-`*` shapes, so users naturally write patterns the excluder ignores.

**Why it matters**: Excludes silently fail closed: a config that intends to skip internal packages still ships them. No diagnostic, no test for non-suffix excludes.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either reject non-supported exclude shapes with an error/warn or implement them
- [ ] #2 Tests for prefix*suffix and bare * exclude shapes
<!-- AC:END -->
