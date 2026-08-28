---
id: TASK-1732
title: 'TEST-5: NodeUnitsProvider::provide and ::name have zero test coverage'
status: To Do
assignee:
  - TASK-1991
created_date: '2026-08-27 11:12'
updated_date: '2026-08-28 14:11'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - extensions-node/about/src/units.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/units.rs:22-33`

**What**: `NodeUnitsProvider` is one of the crate's two registered data providers (`lib.rs:51`), but nothing in the suite ever constructs it. Every test in `units.rs`'s `#[cfg(test)]` module calls the free function `collect_units` or `parse_pnpm_workspace_yaml` directly. A repo-wide grep for `NodeUnitsProvider` returns exactly three hits: the registration line in `lib.rs:51`, the struct definition at `units.rs:22`, and the `impl DataProvider` at `units.rs:24` — no test file.

The untested surface is:
- `DataProvider::name()` returning `PROVIDER_NAME` (`"project_units"`) — the string the registry keys on.
- `DataProvider::provide()`'s `serde_json::to_value(&units).map_err(DataProviderError::from)` — the serialisation step and the error mapping, i.e. the actual JSON shape consumers read.

**Why it matters**: The sister provider in the same crate is covered at exactly this level — `provider_name` and `about_fields_include_homepage` (`lib.rs:158-167`) plus eight `provider.provide(&mut ctx)` round-trips through `serde_json::from_value::<ProjectIdentity>`. The units provider has none of it, so a wrong `PROVIDER_NAME`, a broken `Serialize` on `ProjectUnit`, or a change to the emitted JSON shape would not fail any test in this crate, even though `collect_units` itself is heavily tested. `ops_extension::Context::test_context` is already a dev-dependency (feature `test-support`) and is used throughout `lib.rs`'s tests, so the harness cost is one helper call.

**Notes**: `NodeUnitsProvider` also does not override `about_fields()`, unlike `NodeIdentityProvider`; if the default is intentional a test should say so.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test asserts NodeUnitsProvider.name() == "project_units"
- [ ] #2 A test drives NodeUnitsProvider::provide through ops_extension::Context::test_context over a tempdir workspace and deserialises the result into Vec<ProjectUnit>, asserting member name, path, and version
- [ ] #3 A test covers the empty case: provide over a project with no workspaces returns a JSON array of length 0 rather than null or an error
<!-- AC:END -->
