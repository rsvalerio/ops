---
id: TASK-0554
title: >-
  ERR-1: DetailedDepSpec.default_features missing serde alias for kebab-case
  default-features
status: Triage
assignee: []
created_date: '2026-04-29 05:02'
labels:
  - code-review-rust
  - error-handling
dependencies: []
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/types.rs:399-401`

**What**: DetailedDepSpec declares pub default_features: bool with #[serde(default = "default_true")] but no alias = "default-features". Cargo manifests always use kebab-case (default-features = false); serde silently ignores the unknown key and falls back to true. Sister fields dev_dependencies/build_dependencies/rust_version/license_file/default_run all carry kebab aliases — this one is missing.

**Why it matters**: Every dep that disables default features is parsed as default_features = true, so uses_default_features(), the inheritance merge in resolve_from_detailed_dep, and any deps/feature reports built on this provider give wrong answers for arguably the most common non-default dep shape.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add #[serde(alias = "default-features")] to DetailedDepSpec.default_features
- [ ] #2 Regression test asserting parsing foo = { version = "1", default-features = false } yields default_features == false
<!-- AC:END -->
