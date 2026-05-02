---
id: TASK-0858
title: >-
  API-9: ProjectIdentity sibling structs (LanguageStat etc.) lack
  #[non_exhaustive]
status: Triage
assignee: []
created_date: '2026-05-02 09:19'
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
- [ ] #1 Add non_exhaustive to each of the seven structs and a new(...) constructor for required fields
- [ ] #2 Update internal call sites to use the constructor or struct-update syntax
- [ ] #3 Note the change in any extension-author docs
<!-- AC:END -->
