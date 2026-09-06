---
id: TASK-0059
title: 'TQ-5: config/merge.rs has zero test coverage for core merge logic'
status: Triage
assignee: []
created_date: '2026-04-14 20:54'
labels:
  - rust-test-quality
  - TestGap
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
crates/core/src/config/merge.rs contains 4 functions (merge_field, merge_indexmap, merge_output, merge_config) that form the backbone of configuration layering. None have direct unit tests. While config loader tests exercise merge indirectly, the merge functions themselves are untested — no test verifies that overlay fields properly override base fields, that None overlays leave base unchanged, or that indexmap merging inserts new keys without removing existing ones. merge_config uses destructuring for compile-time exhaustiveness on new fields, but that only ensures the code compiles — not that it behaves correctly.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 merge_field: test that Some overlay replaces base, None overlay preserves base
- [ ] #2 merge_indexmap: test that overlay keys are inserted and existing keys are preserved
- [ ] #3 merge_config: test that each overlay section correctly overrides the corresponding base section
<!-- AC:END -->
