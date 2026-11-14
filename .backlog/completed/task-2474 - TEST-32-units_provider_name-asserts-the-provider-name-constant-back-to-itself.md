---
id: TASK-2474
title: 'TEST-32: units_provider_name asserts the provider-name constant back to itself'
status: Done
assignee: []
created_date: '2026-10-10 15:28'
updated_date: '2026-10-10 22:11'
labels:
  - code-review
  - test
dependencies: []
parent_task_id: 'TASK-2621'
modified_files:
  - extensions-go/about/src/modules.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-32:extensions-go/about/src/modules.rs:units_provider_name'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-go/about/src/modules.rs:581`

**What**: The test's first assertion, `assert_eq!(GoUnitsProvider.name(), PROVIDER_NAME)`, cannot fail: `GoUnitsProvider::name` is defined as `PROVIDER_NAME` (modules.rs:23-25), so the test asserts ground truth back to itself. Only the companion assertion `assert_eq!(PROVIDER_NAME, "project_units")` (modules.rs:582) is live — it pins the cross-stack registry contract against a literal.

**Why it matters**: TEST-32 — an assertion that passes by construction still has to be read and re-approved in every future diff while verifying nothing. Same shape as TASK-2172 (registration-key tests comparing a constant to itself).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The tautological name()-vs-PROVIDER_NAME assertion is removed or replaced with a behavioral assertion
- [x] #2 The contract-pin assertion against the literal "project_units" remains

<!-- AC:END -->
