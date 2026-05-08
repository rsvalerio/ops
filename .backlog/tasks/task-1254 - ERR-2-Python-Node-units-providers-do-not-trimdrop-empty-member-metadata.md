---
id: TASK-1254
title: 'ERR-2: Python/Node units providers do not trim+drop-empty member metadata'
status: To Do
assignee:
  - TASK-1267
created_date: '2026-05-08 13:01'
updated_date: '2026-05-08 13:19'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/units.rs:97` and `extensions-node/about/src/units.rs:46`

**What**: Both providers feed `meta.name`/`meta.version`/`meta.description` straight from the deserialised member manifest into `ProjectUnit`. Whitespace-only fields render as blank About bullets (`unit.version = Some("  ")`), and a whitespace-only name keeps `Some("  ")` so the `format_unit_name` directory fallback never fires.

**Why it matters**: The identity providers already enforce trim_nonempty for name/version/description/license/engines.node (TASK-0563/0813/0814/0704). The units providers for the same stacks bypass that policy, so the workspace card and the identity card disagree on whether a whitespace-only field renders.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Trim and drop empty name/version/description before constructing ProjectUnit in both providers
- [ ] #2 Test: a workspace member with name = "  " falls back to format_unit_name(&member)
- [ ] #3 Sibling test for whitespace-only version/description rendering as None
<!-- AC:END -->
