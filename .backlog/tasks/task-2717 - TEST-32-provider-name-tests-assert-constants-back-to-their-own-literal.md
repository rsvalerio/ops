---
id: TASK-2717
title: 'TEST-32: provider-name tests assert constants back to their own literal'
status: Done
assignee: []
created_date: '2026-11-18 18:32'
updated_date: '2026-11-18 22:20'
labels:
  - code-review-rust
  - TEST
dependencies: []
parent_task_id: 'TASK-2846'
modified_files:
  - extensions-go/about/src/modules.rs
  - extensions-go/about/src/lib.rs
priority: low
ordinal: 1000
dedup_key: 'review-2667.07-7'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:502`, `extensions-go/about/src/lib.rs:148`

**What**: `modules::tests::units_provider_name` asserts `PROVIDER_NAME == "project_units"`, and `lib.rs::tests::provider_name` asserts `GoIdentityProvider.name() == "project_identity"` where `name()` just returns the `DATA_PROVIDER_NAME` constant. Both pass by construction — they restate the literal. The behaviour they claim to guard (both providers land under the cross-stack registry keys) is already pinned by `extension_registers_identity_and_units_providers` (lib.rs:366), which drives the real `register_data_providers` closure and asserts `registry.provider_names() == ["project_identity", "project_units"]` plus distinct payloads.

**Why it matters**: Tautological tests add review surface without catching anything the registry test does not; a rename would have to be re-approved in three places instead of one.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 units_provider_name and provider_name are removed, or replaced by an assertion of behaviour not already covered by extension_registers_identity_and_units_providers
- [x] #2 The registry-level test remains the single pin for the cross-stack provider keys

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Filed by review TASK-2667.07 (code-review-rust run, 2026-11-18).
<!-- SECTION:NOTES:END -->
