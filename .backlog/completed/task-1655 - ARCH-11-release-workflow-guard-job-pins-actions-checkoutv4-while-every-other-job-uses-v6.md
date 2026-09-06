---
id: TASK-1655
title: >-
  ARCH-11: release-workflow-guard job pins actions/checkout@v4 while every other
  job uses @v6
status: Done
assignee: []
created_date: '2026-06-07 11:06'
updated_date: '2026-06-07 11:32'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `.github/workflows/ci.yml:116`

**What**: The new `release-workflow-guard` job checks out with `actions/checkout@v4`; every other checkout in `ci.yml`, `bump.yml`, and `release.yml` uses `actions/checkout@v6`.

**Why it matters**: Same drift class as ARCH-11 (diverging dependency versions across a workspace, applied to CI config): two majors of the same action in one repo means security/back-compat updates get applied to one pin and missed on the other, and Dependabot/renovate-style bumps produce inconsistent diffs. The guard job only greps a file, so aligning to `@v6` is risk-free.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 release-workflow-guard uses the same actions/checkout major (@v6) as the rest of the workflows
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Aligned actions/checkout to @v6 in ci.yml. Note: all 8 ci.yml checkouts (not just release-workflow-guard) were on @v4; all were bumped so the repo now uses a single checkout major across ci.yml, bump.yml, and release.yml.
<!-- SECTION:NOTES:END -->
