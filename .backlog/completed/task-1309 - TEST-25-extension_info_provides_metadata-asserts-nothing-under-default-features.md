---
id: TASK-1309
title: >-
  TEST-25: extension_info_provides_metadata asserts nothing under default
  features
status: Done
assignee:
  - TASK-1383
created_date: '2026-05-11 19:58'
updated_date: '2026-05-12 23:16'
labels:
  - code-review-rust
  - test-quality
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/cli/src/registry/tests.rs:130-148`

**What**: The test loops `for info in &infos { assert!(...) }` but `infos` is built from `builtin_extensions(&Config::default(), ...)`, which returns an empty `Vec` when no stack-* feature flags are active (the default `cargo test -p ops` shape). The loop runs zero iterations and the test silently passes.

**Why it matters**: This is the same anti-pattern that `register_extension_commands_aggregates_across_multiple_extensions` (registry/tests.rs:473-525, TASK-1301) fixed by stubbing in-process ExtA/ExtB. The current test reports coverage on the dashboard but provides zero regression protection under default-feature builds.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Test uses inline stub extensions (like TASK-1301's ExtA/ExtB) so it exercises collect_extension_info regardless of feature flags
- [ ] #2 Add precondition assert!(!infos.is_empty()) so a future regression that drops collect_extension_info entries is caught
<!-- AC:END -->
