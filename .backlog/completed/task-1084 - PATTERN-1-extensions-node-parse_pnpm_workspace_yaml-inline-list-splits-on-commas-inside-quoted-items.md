---
id: TASK-1084
title: >-
  PATTERN-1: extensions-node parse_pnpm_workspace_yaml inline-list splits on
  commas inside quoted items
status: Done
assignee: []
created_date: '2026-05-07 21:21'
updated_date: '2026-05-08 06:16'
labels:
  - code-review-rust
  - PATTERN
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:191-200`

**What**: `parse_pnpm_workspace_yaml` for the inline-list shape does `inner.split(',')` then `unquote` each piece. A quoted entry containing a literal `,` (e.g. `"a,b"` for a directory name with a comma) is shredded. YAML allows commas inside flow-quoted scalars.

**Why it matters**: pnpm-workspace.yaml is user-authored; pnpm itself parses these via a real YAML parser. The naive split is a parser-shape divergence that silently produces wrong globs without diagnostic — exactly the failure mode the existing `saw_packages_key` debug log was added to surface for the block-scalar case.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Inline-list parsing respects quote boundaries when splitting on ','
- [ ] #2 Regression test pins packages: ['a,b', 'c'] → two items
- [ ] #3 Docstring lists this shape under 'supported'
<!-- AC:END -->
