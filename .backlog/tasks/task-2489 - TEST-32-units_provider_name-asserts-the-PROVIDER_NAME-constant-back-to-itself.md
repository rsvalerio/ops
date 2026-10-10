---
id: TASK-2489
title: 'TEST-32: units_provider_name asserts the PROVIDER_NAME constant back to itself'
status: To Do
assignee: []
created_date: '2026-10-10 15:30'
updated_date: '2026-10-10 21:12'
labels:
  - code-review
  - test
dependencies: []
parent_task_id: 'TASK-2621'
modified_files:
  - extensions-python/about/src/units.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-32:extensions-python/about/src/units.rs:units_provider_name'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/units.rs:433`

**What**: `units_provider_name` asserts `PythonUnitsProvider.name() == PROVIDER_NAME` (the impl body is `PROVIDER_NAME` — ground truth to itself) and then `PROVIDER_NAME == "project_units"` (the constant against its own defining literal). Both pass by construction: the first can never fail, and the second only fails if the const and the test literal are edited inconsistently.

**Why it matters**: TEST-32 bans tests that assert ground truth back to itself; the registry-key contract this test documents is already pinned behaviourally and end-to-end by `extension_registers_identity_and_units_providers` (lib.rs), which asserts `provider_names() == ["project_identity", "project_units"]` against literals independent of the constant. A coordinated rename fails that test, so this one adds no failure mode anyone cares about — only a second place to read and re-approve.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The tautological assertions are removed, or the test is rewritten to assert a property independent of the constant's definition while the registry test keeps the end-to-end key contract pinned
- [ ] #2 cargo test -p ops-about-python still fails on an accidental rename of the project_units registry key (via the registry-level test)
<!-- AC:END -->
