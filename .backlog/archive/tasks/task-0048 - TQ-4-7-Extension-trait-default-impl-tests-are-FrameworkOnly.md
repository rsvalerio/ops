---
id: TASK-0048
title: 'TQ-4: 7 Extension trait default-impl tests are FrameworkOnly'
status: Triage
assignee: []
created_date: '2026-04-14 20:22'
labels:
  - rust-test-quality
  - FrameworkOnly
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
crates/extension/src/tests.rs — Seven tests (extension_default_description_is_empty, extension_default_shortname_equals_name, extension_default_types_is_empty, extension_default_command_names_is_empty, extension_default_data_provider_name_is_none, extension_default_stack_is_none, extension_default_register_data_providers_is_noop) only verify default trait return values without testing any project logic. Each asserts a single hardcoded default (empty string, None, empty vec). They add noise to the suite and inflate coverage metrics without catching regressions. Rules: TEST-25, TEST-11.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Default-impl tests are either removed or enhanced to test meaningful behavior through the trait
<!-- AC:END -->
