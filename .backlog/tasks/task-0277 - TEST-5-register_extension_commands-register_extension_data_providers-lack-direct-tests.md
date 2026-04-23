---
id: TASK-0277
title: >-
  TEST-5: register_extension_commands/register_extension_data_providers lack
  direct tests
status: Done
assignee: []
created_date: '2026-04-23 06:37'
updated_date: '2026-04-23 15:24'
labels:
  - rust-code-review
  - test
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry.rs:116`

**What**: Only exercised transitively; empty-input and multi-extension aggregation paths uncovered.

**Why it matters**: Public helpers without dedicated tests.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Add register_with_empty_slice_is_noop
- [ ] #2 Add register_with_multiple_extensions_aggregates
<!-- AC:END -->
