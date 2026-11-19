---
id: TASK-2725
title: 'DUP-3: Provider-name string literals repeated instead of shared consts'
status: To Do
assignee: []
created_date: '2026-11-18 18:34'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - duplication
dependencies: []
parent_task_id: 'TASK-2848'
modified_files:
  - extensions-java/about/src/lib.rs
  - extensions-java/about/src/maven/mod.rs
  - extensions-java/about/src/gradle/mod.rs
priority: low
ordinal: 1000
dedup_key: 'DUP-3:extensions-java/about/src/lib.rs:register_data_providers'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-java/about/src/lib.rs:56`, `extensions-java/about/src/lib.rs:58`, `extensions-java/about/src/lib.rs:60`, `extensions-java/about/src/lib.rs:89`, `extensions-java/about/src/lib.rs:91`, `extensions-java/about/src/lib.rs:93`, `extensions-java/about/src/maven/mod.rs:21`, `extensions-java/about/src/maven/mod.rs:64`, `extensions-java/about/src/gradle/mod.rs:40`, `extensions-java/about/src/gradle/mod.rs:317`

**What**: The registry keys `"project_identity"` (6 sites) and `"project_units"` (4 sites) are spelled as inline literals across the `impl_extension!` blocks (`data_provider_name`, `register`) and each provider's `DataProvider::name()`. Every sibling stack crate names them once (`DATA_PROVIDER_NAME` / `PROVIDER_NAME` in extensions-go, -node, -python, -rust; `ops_about::units::PROJECT_UNITS_PROVIDER` exists for the units key). The Java crate is the outlier.

**Why it matters**: The register key and `name()` must agree; a typo in one literal registers a provider the About page never queries, and nothing at compile time links the copies.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Each registry key is defined once as a const (or reuses ops_about's PROJECT_UNITS_PROVIDER) and every register call, data_provider_name and DataProvider::name uses it
- [ ] #2 No inline "project_identity" / "project_units" literal remains in non-test code of ops-about-java
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.08 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
