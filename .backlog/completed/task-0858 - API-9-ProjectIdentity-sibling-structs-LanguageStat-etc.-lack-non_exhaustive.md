---
id: TASK-0858
title: >-
  API-9: ProjectIdentity sibling structs (LanguageStat etc.) lack
  #[non_exhaustive]
status: Done
assignee: []
created_date: '2026-05-02 09:19'
updated_date: '2026-05-02 14:37'
labels:
  - code-review-rust
  - api-design
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/project_identity.rs:103-210`

**What**: ProjectIdentity is correctly non_exhaustive (line 32) but the seven sibling structs around it (LanguageStat, ProjectUnit, CoverageStats, UnitCoverage, ProjectCoverage, UnitDeps, ProjectDependencies) are not, even though they are part of the same JSON contract returned by stack-specific data providers and can grow fields over time.

**Why it matters**: API-9 - adding a field to any of them breaks downstream extensions that construct via struct literal. The crate has set the precedent on ProjectIdentity; the rest should follow for consistency.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add non_exhaustive to each of the seven structs and a new(...) constructor for required fields
- [x] #2 Update internal call sites to use the constructor or struct-update syntax
- [x] #3 Note the change in any extension-author docs
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Added #[non_exhaustive] to LanguageStat (already had ::new), ProjectUnit, CoverageStats, UnitCoverage, ProjectCoverage, UnitDeps, ProjectDependencies. Added ::new constructors for the six that lacked one. Updated all internal cross-crate call sites to use ::new (extensions-rust/coverage_provider/deps_provider/units, extensions-python/about/units, extensions-node/about/units, extensions-go/about/modules) and the test sites in extensions/about (cards, coverage, deps, units, lib). 85 tests in ops-about pass; ops verify clean.
<!-- SECTION:NOTES:END -->
