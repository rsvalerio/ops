---
id: TASK-2172
title: >-
  TEST-32: both registration-key tests in create-review-tasks-rust compare a
  constant to itself and cannot fail
status: To Do
assignee:
  - TASK-2240
created_date: '2026-09-08 07:11'
updated_date: '2026-09-08 10:56'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - extensions-rust/create-review-tasks/src/provider.rs
  - extensions-rust/create-review-tasks/src/lib.rs
priority: medium
ordinal: 85000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/create-review-tasks/src/provider.rs:256-262`, `extensions-rust/create-review-tasks/src/lib.rs:20,54-59`

**What**: Two tests document themselves as guards against a registration-key mismatch, but both sides of each assertion resolve to the same constant, so neither can ever fail.

1. `provider_name_matches_the_engines_registration_key` asserts `RustReviewTargetsProvider.name() == ops_create_review_tasks::DATA_PROVIDER_NAME`, while the impl body (provider.rs:36-38) is literally `ops_create_review_tasks::DATA_PROVIDER_NAME`. The assertion is `X == X` by construction.

2. `extension_registers_the_review_targets_provider_under_the_engine_key` asserts `registry.provider_names().contains(&DATA_PROVIDER_NAME)`, where the crate-local `DATA_PROVIDER_NAME` (lib.rs:20) is defined as `ops_create_review_tasks::DATA_PROVIDER_NAME` — the same value the `register_data_providers` closure registers under (lib.rs:35-38). The doc comment claims it catches "a mismatch [that] surfaces at runtime as 'the detected stack has no create-review-tasks extension compiled in'", but the mismatch it describes is unrepresentable while both sites read one constant.

The second half of test 2 — decoding the payload and asserting `payload["skill"] == SKILL_NAME` — is a real behavioural assertion and should stay.

**Why it matters**: TEST-32: the assertions exist, are unique, and are still worthless; they have to be read and re-approved in every future diff while covering nothing. Worse, they read as coverage for the cross-crate key contract, so the genuinely unguarded risk — the engine looking the provider up under a key it computes some other way — looks tested when it is not.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 provider_name_matches_the_engines_registration_key is either deleted or rewritten to assert a property that is not true by construction (e.g. the engine's own lookup path resolves this provider)
- [ ] #2 extension_registers_the_review_targets_provider_under_the_engine_key no longer asserts presence of a key against the same constant used to register it; its behavioural half (payload skill == SKILL_NAME) is preserved
- [ ] #3 Doc comments on the retained tests describe what they actually verify, with no claim of covering a mismatch that cannot occur
<!-- AC:END -->
