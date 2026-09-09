---
id: TASK-2201
title: 'TEST-5: PythonUnitsProvider''s DataProvider impl and the about-python provider registration are never exercised'
status: Done
assignee: []
created_date: '2026-09-08 07:18'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - tests
dependencies: []
parent_task_id: 'TASK-2240'
modified_files:
  - extensions-python/about/src/units.rs
  - extensions-python/about/src/lib.rs
priority: medium
ordinal: 114000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/units.rs:17`, `extensions-python/about/src/lib.rs:51`

**What**: Every test in `units.rs` calls the free function `collect_units` directly. Nothing in the crate ever calls `PythonUnitsProvider::name()`, `PythonUnitsProvider::about_fields()`, or `PythonUnitsProvider::provide()`, and nothing exercises the `register_data_providers` closure in `lib.rs` (`registry.register(units::PROVIDER_NAME, ...)`) or the `PYTHON_ABOUT_FACTORY` entry. The identity provider is better off — `identity_at` drives `provide()` end to end and there are `provider_name` / `about_fields_include_homepage` tests — but the units half has none of that.

The sibling Node crate pins exactly this surface (`extensions-node/about/src/units.rs:836-880`: `units_provider_name`, `units_provider_declares_no_about_fields`, `units_provider_serialises_workspace_members`, `units_provider_empty_workspace_is_empty_array`), so the gap is specific to the Python crate.

**Why it matters**: A typo in `PROVIDER_NAME`, a dropped `registry.register` line (its `Result` is discarded with `let _ =`), or a regression in the `serde_json::to_value` step silently unregisters or empties the Python workspace card, and the whole suite stays green. The JSON shape consumers actually read is never asserted — only the in-memory `Vec<ProjectUnit>`.

**Twin**: TASK-2184 files the same finding against `extensions-go/about` (`GoUnitsProvider`). Fix both to the Node pattern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test asserts PythonUnitsProvider.name() == "project_units"
- [ ] #2 A test drives PythonUnitsProvider::provide() against a uv workspace tempdir and asserts the deserialised JSON payload (name, path, version, description)
- [ ] #3 A test asserts the no-workspace case serialises to an empty JSON array, not null
- [ ] #4 A test exercises AboutPythonExtension's register_data_providers and asserts both project_identity and project_units are registered
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-2240. units.rs: units_provider_name, units_provider_serialises_workspace_members, units_provider_no_workspace_is_empty_array (AC1-3, Node pattern). lib.rs: extension_registers_identity_and_units_providers (AC4, distinct payload shapes).
<!-- SECTION:NOTES:END -->
