---
id: TASK-0707
title: >-
  ARCH-6: cargo-toml resolve_package_inheritance does not propagate
  readme/keywords/categories despite cargo declaring them workspace-inheritable
status: Done
assignee:
  - TASK-0736
created_date: '2026-04-30 05:29'
updated_date: '2026-04-30 11:50'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/cargo-toml/src/inheritance.rs:43-66`

**What**: `resolve_package_inheritance` substitutes inherited values for `version`, `edition`, `rust_version`, `description`, `documentation`, `homepage`, `repository`, `license`, and `authors`. Cargo also documents `readme`, `keywords`, `categories`, `publish`, `license-file`, and `include`/`exclude` as workspace-inheritable, and ops_cargo_toml stores `readme: Option<ReadmeSpec>` (line 122 of `types.rs`) and `keywords` / `categories` as plain `Vec<String>` — none of which get pulled from `[workspace.package]`. A consumer that reads `package.readme` from the data provider on a member crate that inherits via `readme = { workspace = true }` sees `None` even when the workspace defines one.

**Why it matters**: Pairs with the API-1 finding above — even after the type changes, the inheritance step needs new arms or the data provider reports stale/empty fields for downstream About / metadata flows. Filed as ARCH-6 (incomplete feature implementation) rather than PATTERN-1 because the existing arms are correct, just not exhaustive.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 resolve_package_inheritance substitutes readme/keywords/categories/publish/license-file from [workspace.package] when the local field is in the Inherited state
- [x] #2 table or shared loop drives the inheritance arms so adding a new field does not require touching three places
- [x] #3 test pins a member crate inheriting all of readme, keywords, categories, license-file from [workspace.package]
<!-- AC:END -->
