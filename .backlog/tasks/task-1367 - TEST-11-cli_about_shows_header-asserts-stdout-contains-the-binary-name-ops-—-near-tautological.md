---
id: TASK-1367
title: >-
  TEST-11: cli_about_shows_header asserts stdout contains the binary name 'ops'
  — near-tautological
status: Done
assignee:
  - TASK-1384
created_date: '2026-05-12 21:29'
updated_date: '2026-05-12 23:34'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/tests/integration.rs:631`

**What**: The test asserts stdout contains the substring `"ops"`. The binary itself is `ops`, and unrelated outputs (help, errors, version banners) all contain it. The assertion is near-tautological for any successful binary run.

**Why it matters**: TEST-11: a regression where the about header silently failed to render (and the binary printed only help/version) would still satisfy `contains("ops")`. Replace with a stable about-header marker (project banner, section heading, or a known [about] field name).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Assert a stable about-card marker (project name field, banner string, or section header) instead of the substring 'ops'
- [ ] #2 Alternatively assert at least one [about] field name (e.g. 'workspace' / 'languages') renders to stdout
<!-- AC:END -->
