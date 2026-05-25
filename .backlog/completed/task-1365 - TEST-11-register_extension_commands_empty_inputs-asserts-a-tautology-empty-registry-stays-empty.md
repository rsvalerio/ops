---
id: TASK-1365
title: >-
  TEST-11: register_extension_commands_empty_inputs asserts a tautology (empty
  &[] -> registry stays empty)
status: Done
assignee:
  - TASK-1385
created_date: '2026-05-12 21:29'
updated_date: '2026-05-17 09:33'
labels:
  - code-review-rust
  - tests
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/tests.rs:286`

**What**: The test passes `&[]` to `register_extension_commands` and asserts `is_empty()`. Passing zero extensions cannot mutate the registry, so the post-condition is provable at the type level. The only failure mode would be a panic on empty input, which other tests already cover by exercising the function with non-empty inputs.

**Why it matters**: TEST-11 vacuous property. Sibling TASK-1280 already closed `register_extension_data_providers_empty_inputs` for the same reason. Either drop this test or fold it into a property check that calls the function on a pre-populated registry and asserts existing entries survive (a real preservation property).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Drop the test, or replace it with a check that pre-populates the registry and verifies entries survive untouched after a zero-extension call
- [ ] #2 Add a panic-on-empty smoke (one-liner) if keeping the call site exercised — but only if it's not already covered elsewhere
<!-- AC:END -->
