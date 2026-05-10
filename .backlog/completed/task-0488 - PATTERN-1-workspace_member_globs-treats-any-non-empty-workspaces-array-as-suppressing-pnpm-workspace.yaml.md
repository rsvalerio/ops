---
id: TASK-0488
title: >-
  PATTERN-1: workspace_member_globs treats any non-empty workspaces array as
  suppressing pnpm-workspace.yaml
status: Done
assignee:
  - TASK-0532
created_date: '2026-04-28 06:08'
updated_date: '2026-04-28 15:44'
labels:
  - code-review-rust
  - pattern
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:65-109`

**What**: After parsing package.json, the function only consults pnpm-workspace.yaml when patterns.is_empty(). A package.json with `"workspaces": ["!packages/legacy"]` (exclude-only) leaves patterns non-empty, so pnpm-workspace.yaml is silently ignored even though no positive include exists. Includes and excludes are not separated until resolve_member_globs runs later, by which point the pnpm fallback is already skipped.

**Why it matters**: Real repos sometimes carry both a stub workspaces field in package.json and the canonical layout in pnpm-workspace.yaml. The current heuristic conflates "package.json had any workspaces entries" with "package.json declared positive includes", producing an empty unit list with no diagnostic. Decide and encode the precedence rule explicitly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Split includes/excludes inside workspace_member_globs and treat exclude-only arrays as empty for the pnpm fallback decision
- [ ] #2 Document the precedence rule in the file-level comment (npm/yarn shadows pnpm, or they merge)
- [ ] #3 Regression test: package.json with workspaces ['!packages/x'] plus a real pnpm-workspace.yaml resolves the pnpm members
<!-- AC:END -->
