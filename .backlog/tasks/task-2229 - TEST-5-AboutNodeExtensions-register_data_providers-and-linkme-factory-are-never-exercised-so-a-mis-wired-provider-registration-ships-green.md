---
id: TASK-2229
title: >-
  TEST-5: AboutNodeExtension's register_data_providers and linkme factory are
  never exercised, so a mis-wired provider registration ships green
status: To Do
assignee:
  - TASK-2240
created_date: '2026-09-08 07:23'
updated_date: '2026-09-08 10:55'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-node/about/src/lib.rs
priority: medium
ordinal: 135000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-node/about/src/lib.rs:41`

**What**: The `ops_extension::impl_extension!` block is the crate's only wiring
to the rest of `ops`: it declares `NAME`/`SHORTNAME`/`stack`, registers both
data providers, and installs the `NODE_ABOUT_FACTORY` linkme slice entry.
Nothing in the crate's tests touches any of it.

The existing tests call the providers directly
(`NodeIdentityProvider.provide(...)`, `units_provider_serialises_workspace_members`),
which is precisely the path that keeps working when the wiring is wrong. Not
covered:

- `register_data_providers` actually installing both providers under
  `"project_identity"` and `"project_units"`;
- the two `let _ = registry.register(...)` calls, which discard the
  `Option<Box<dyn DataProvider>>` that `DataRegistry::register`
  (`crates/extension/src/data.rs:405`) returns to signal *rejected as a
  duplicate*. Under first-write-wins a name collision silently drops this
  crate's provider and the About card loses a section with no failure;
- `NODE_ABOUT_FACTORY` yielding an extension, and its declared metadata
  (`Stack::Node`, `ExtensionType::DATASOURCE`, `data_provider_name`).

**Why it matters**: every failure mode here is silent — a typo'd provider key,
a stack mismatch, or a duplicate registration removes About output rather than
producing an error. `units.rs:834` already documents that risk for
`PROVIDER_NAME` and pins the constant, but nothing pins the registration that
consumes it.

**Twins** (same class, other stacks — cross-reference, do not merge):
TASK-2184 (about-go), TASK-2201 (about-python).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 a test drives AboutNodeExtension::register_data_providers against a real DataRegistry and asserts both project_identity and project_units resolve
- [ ] #2 a test asserts the register calls report no duplicate rejection (the discarded Option is None for both)
- [ ] #3 a test exercises NODE_ABOUT_FACTORY and asserts the extension's name, shortname, stack and type
<!-- AC:END -->
