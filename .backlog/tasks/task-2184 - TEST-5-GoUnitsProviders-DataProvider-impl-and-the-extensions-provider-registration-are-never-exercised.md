---
id: TASK-2184
title: 'TEST-5: GoUnitsProvider''s DataProvider impl and the extension''s provider registration are never exercised'
status: Done
assignee: []
created_date: '2026-09-08 07:13'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2240'
modified_files:
  - extensions-go/about/src/modules.rs
  - extensions-go/about/src/lib.rs
priority: medium
ordinal: 97000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:15` (`GoUnitsProvider`), `extensions-go/about/src/lib.rs:45` (`register_data_providers`)

**What**: Every `modules.rs` test calls the private `collect_units(dir)` helper directly. Nothing calls `GoUnitsProvider::provide` or `GoUnitsProvider::name`, so the code that actually runs in production — `serde_json::to_value(&units)` plus the `DataProviderError::from` mapping, and the `ctx.working_directory()` plumbing — has zero coverage. The sibling `GoIdentityProvider` is covered end-to-end through `provide` in `lib.rs` (nine tests), which makes the gap asymmetric within the same crate.

The `register_data_providers` closure in `impl_extension!` is likewise untested: nothing asserts that both `project_identity` and `project_units` land in a `DataRegistry`, or that the registered `project_units` name matches `modules::PROVIDER_NAME`. A rename or a dropped `registry.register` line would compile and ship silently — `register` returns `Option` and the result is discarded with `let _ =`.

**Why it matters**: The Go units card can regress to an empty/`null` payload or an unregistered provider without a single test failing. This is the Go-stack twin of TASK-2154 (`per_crate_units` / `RustCoverageProvider::provide` have no happy-path test) — the same coverage shape is missing in both stacks; fix them consistently.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test drives GoUnitsProvider::provide through a Context::test_context over a go.work fixture and deserializes the returned Value back into Vec<ProjectUnit>, asserting names, paths and versions
- [ ] #2 A test asserts GoUnitsProvider::name() == modules::PROVIDER_NAME == "project_units"
- [ ] #3 A test builds a DataRegistry through the extension's register_data_providers and asserts both project_identity and project_units are present
- [ ] #4 Cross-referenced with TASK-2154 so the Rust and Go stacks land the same provider-level coverage shape
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-2240. modules.rs: units_provider_name, units_provider_serialises_go_work_modules, units_provider_empty_project_is_empty_array. lib.rs: extension_registers_identity_and_units_providers (both keys, distinct payload shapes). Cross-referenced with TASK-2154 provider_tests per AC #4.
<!-- SECTION:NOTES:END -->
