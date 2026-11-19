---
id: TASK-2728
title: 'TEST-32: provider_name test asserts DATA_PROVIDER_NAME back to its own literal'
status: To Do
assignee: []
created_date: '2026-11-18 18:35'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2848'
modified_files:
  - extensions-python/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-32:extensions-python/about/src/lib.rs:tests::provider_name'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-python/about/src/lib.rs:567` (test `provider_name`)

**What**: `provider_name` asserts `PythonIdentityProvider.name() == "project_identity"`, but `name()` just returns the `DATA_PROVIDER_NAME` const (`lib.rs:33`), whose value is that same literal. The test passes by construction. The meaningful contract (both providers registered under `project_identity` / `project_units`, and each answering with its own payload) is already pinned by `extension_registers_identity_and_units_providers` (`lib.rs:1258`), which asserts `registry.provider_names()` against the literal keys and exercises `registry.provide("project_identity", ..)`.

**Why it matters**: A tautological assertion can never fail for a reason anyone cares about, overlaps a stronger test, and still has to be read and maintained in every future diff.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 The provider_name test is removed, or replaced by an assertion of behaviour the name drives (e.g. registry lookup) that is not already covered by extension_registers_identity_and_units_providers
- [ ] #2 cargo test -p ops-about-python passes
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.10 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
