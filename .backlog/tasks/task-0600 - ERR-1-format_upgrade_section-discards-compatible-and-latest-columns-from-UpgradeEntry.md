---
id: TASK-0600
title: >-
  ERR-1: format_upgrade_section discards compatible and latest columns from
  UpgradeEntry
status: Done
assignee:
  - TASK-0638
created_date: '2026-04-29 05:19'
updated_date: '2026-04-29 10:40'
labels:
  - code-review-rust
  - ERR
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/deps/src/format.rs:84`

**What**: UpgradeEntry parses six fields (name, old_req, compatible, latest, new_req, note), but the formatter prints only `{name} {old_req} -> {new_req}`. Compatible-version-cap and absolute-latest-version are dropped silently — for breaking upgrades the operator never sees how far behind they are (e.g. cap stuck at 3.x while latest is 5.x).

**Why it matters**: The whole point of categorising compatible vs breaking is to surface those gaps; the renderer makes them invisible.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Breaking-upgrade rows render at minimum old_req, latest, and new_req
- [ ] #2 Test asserts format_report for incompatible entry includes the latest value
<!-- AC:END -->
