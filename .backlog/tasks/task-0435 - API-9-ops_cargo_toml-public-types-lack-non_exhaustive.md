---
id: TASK-0435
title: 'API-9: ops_cargo_toml public types lack #[non_exhaustive]'
status: Done
assignee:
  - TASK-0533
created_date: '2026-04-28 04:43'
updated_date: '2026-04-28 17:49'
labels:
  - code-review-rust
  - api
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/types.rs:28,91,232,259,367`

**What**: CargoToml, Package, Workspace, WorkspacePackage, and DetailedDepSpec are pub and re-exported from ops_cargo_toml::lib.rs:67, but none are annotated #[non_exhaustive]. Cargo manifest schema gains fields routinely (recently lints, cargo-features, profile-related additions), and any future field addition here is a SemVer-breaking change for downstream consumers who construct these structs.

**Why it matters**: Consistent with TASK-0167 / TASK-0235 / TASK-0349 which closed the same gap on other public extension-facing types. The cargo_toml data provider is the canonical example of a data-source extension (lib.rs comment), so its public types are heavily depended on.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add #[non_exhaustive] to each of CargoToml, Package, Workspace, WorkspacePackage, DetailedDepSpec
- [x] #2 Add public builders / Default impls where current call sites construct them by struct-literal so external users (and tests) keep compiling
- [x] #3 Verify deserialization still works (serde ignores #[non_exhaustive] on input)
<!-- AC:END -->
