---
id: TASK-0480
title: 'ERR-1: parse_pnpm_workspace_yaml ignores !-prefixed exclusion entries'
status: Done
assignee:
  - TASK-0532
created_date: '2026-04-28 05:48'
updated_date: '2026-04-28 15:44'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:117-148`

**What**: The pnpm workspace YAML parser ignores `!`-prefixed exclusion entries — it pushes them into the includes list as-is. resolve_member_globs (called from collect_units) only splits exclusions when reading the package.json `workspaces` field, not when patterns came from pnpm-workspace.yaml.

**Why it matters**: pnpm officially supports `- \'!packages/internal-*\'` to exclude paths from a workspace; the literal `!` prevents the glob from ever matching, so the entry is silently dropped (no error, just incorrect behaviour).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Either parse exclusions inside parse_pnpm_workspace_yaml (return them separately) or thread the ! split through resolve_member_globs so pnpm paths receive the same exclusion handling as npm/yarn workspaces
- [ ] #2 Add a test with pnpm-workspace.yaml containing both an include glob and a !-prefixed exclude, asserting the excluded directory does not appear in the resulting units
<!-- AC:END -->
