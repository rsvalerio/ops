---
id: TASK-0167
title: >-
  API-9: public identity/data types are not #[non_exhaustive] despite being
  extension-facing
status: To Do
assignee: []
created_date: '2026-04-22 21:24'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - API
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: crates/core/src/project_identity.rs:29-159

**What**: ProjectIdentity, LanguageStat, ProjectUnit, CoverageStats, UnitCoverage, ProjectCoverage, UnitDeps, ProjectDependencies are all pub structs with multiple pub fields and no #[non_exhaustive]. They are the canonical data contract between ops-core and stack-specific extensions (docs note "Stack-specific extensions provide a project_identity data provider returning a ProjectIdentity as JSON"). Downstream extensions construct these with StructLiteral syntax and pattern-match on them, so adding a field is a breaking change despite #[serde(default)] softening JSON compatibility.

**Why it matters**: API-9. Marking these #[non_exhaustive] preserves forward compatibility: internal ops-core can add fields without cargo-semver-checks flagging a major-version break, and external extension crates continue to compile via ..Default::default() / ..unit update syntax. The Default derives make this practical. See also same pattern in extension/src/data.rs (DataField, DataProviderSchema).

**Notes**: Also applies to DataField and DataProviderSchema in crates/extension/src/data.rs:11-23, which are re-used by the data_field! macro across every stack extension.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Mark ProjectIdentity, LanguageStat, ProjectUnit, CoverageStats, UnitCoverage, ProjectCoverage, UnitDeps, ProjectDependencies with #[non_exhaustive]
- [ ] #2 Mark DataField and DataProviderSchema with #[non_exhaustive]
- [ ] #3 Update internal construction sites to use ..Default::default() where needed
<!-- AC:END -->
