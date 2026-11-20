---
id: TASK-2891
title: 'code-review-plan-wave33'
status: To Do
assignee: []
created_date: '2026-11-20 13:26'
updated_date: '2026-11-20 13:26'
labels:
  - code-review-wave
dependencies:
  - TASK-2887
  - TASK-2889
  - TASK-2890
modified_files:
  - extensions/about/src/machine/layers.rs
  - extensions/about/Cargo.toml
  - extensions/about/src/cards.rs
  - extensions/about/src/machine/cfg_expr.rs
  - extensions/about/src/manifest_cache.rs
  - extensions/about/src/manifest_io.rs
  - crates/extension/src/context.rs
  - extensions-rust/foundation/src/lib.rs
  - extensions/about/src/lib.rs
  - extensions/about/src/deps.rs
  - crates/extension/src/provider_names.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave33
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
ops-about hygiene: reuse the ops_core hardened capped read, stop public docs linking private items (ops-about, ops-extension, ops-rust-foundation), and name warmed providers via shared constants.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: none
<!-- SECTION:NOTES:END -->
