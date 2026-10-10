---
id: TASK-2439
title: 'code-review-plan-wave51'
status: To Do
assignee: []
created_date: '2026-10-10 14:31'
updated_date: '2026-10-10 14:32'
labels:
  - code-review-wave
dependencies:
  - TASK-2426
  - TASK-2429
  - TASK-2431
  - TASK-2432
  - TASK-2436
modified_files:
  - extensions-rust/about/src/manifest.rs
  - extensions-rust/about/src/manifest_cache.rs
  - extensions-rust/about/src/members.rs
  - extensions-rust/about/src/units.rs
  - extensions-rust/about/src/coverage_provider.rs
  - crates/extension/src/error.rs
  - crates/extension/tests/public_api.rs
  - extensions/sqlite/src/connection.rs
  - extensions/sqlite/src/error.rs
  - extensions/sqlite/src/sql/query/helpers.rs
  - extensions/sqlite/src/sql/query/coverage.rs
  - extensions/sqlite/src/sql/ingest/orchestrator.rs
  - extensions/about/src/cards.rs
  - extensions/about/src/identity.rs
  - extensions/about/src/providers.rs
  - extensions/about/src/deps.rs
  - extensions/about/src/machine.rs
  - extensions/about/src/code.rs
  - extensions/about/src/loc.rs
  - extensions/about/src/coverage.rs
  - extensions/about/src/units.rs
  - extensions/about/src/test_support.rs
  - extensions/about/Cargo.toml
  - crates/theme/src/style/strip.rs
  - crates/theme/src/configurable/report.rs
  - crates/theme/src/resolve.rs
  - crates/theme/src/lib.rs
  - crates/theme/src/configurable.rs
  - crates/theme/src/configurable/boxed.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave51
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
All five are the same concern: comments and docs that narrate change history (previous/pre-fix/legacy shapes, TASK-NNNN and rule-ID provenance tags) or cite facts that are no longer true. Continuations of TASK-2374/2408/2397/2352/2361 into the files those tasks did not list. TASK-2429 and TASK-2436 share crates/extension/tests/public_api.rs.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2441 (extensions/sqlite/src/error.rs)
<!-- SECTION:NOTES:END -->
