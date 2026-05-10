---
id: TASK-0813
title: >-
  ERR-2: package.json LicenseField text/object branches are not
  trim/empty-filtered
status: Done
assignee:
  - TASK-0823
created_date: '2026-05-01 06:03'
updated_date: '2026-05-01 09:21'
labels:
  - code-review-rust
  - errors
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/package_json.rs:107-110`

**What**: Sister fields (name, version, homepage, description) all funnel through trim_nonempty; the license text branches return raw s and raw r#type directly.

**Why it matters**: Mirrors the gap closed for pyproject in TASK-0704 — a license value of all-whitespace package.json renders a blank license bullet on the About card.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Wrap both branches in trim_nonempty(Some(...))
- [ ] #2 Test for license whitespace and license object with whitespace type rendering as no license
<!-- AC:END -->
