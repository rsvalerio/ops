---
id: TASK-0684
title: >-
  ERR-4: parse_pnpm_workspace_yaml silently produces empty list for unrecognised
  shapes
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:15'
updated_date: '2026-04-30 08:45'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:135-178`

**What**: `parse_pnpm_workspace_yaml` silently produces an empty list for any shape it doesn't recognise (block scalar, anchored list, packages defined inside another mapping such as `pnpm:` wrapper, indentation switches), with no tracing::warn!/debug! saying "we found a packages: key but produced 0 entries".

**Why it matters**: When a real pnpm workspace fails to surface, the symptom is "no project units" with no diagnostic — operators can't tell parse skipping from genuine "no members". Sister parsers (package.json parse, go.mod read) all emit a tracing event when they bail.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 When packages: is matched but out ends empty, emit a tracing::debug! with the file path and a 'no entries parsed' message
- [x] #2 Test asserts the trace event fires for an unsupported shape (e.g. block scalar packages: |\n  apps/*)
<!-- AC:END -->
