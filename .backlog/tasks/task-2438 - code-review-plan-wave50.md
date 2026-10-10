---
id: TASK-2438
title: 'code-review-plan-wave50'
status: To Do
assignee: []
created_date: '2026-10-10 14:31'
updated_date: '2026-10-10 14:32'
labels:
  - code-review-wave
dependencies:
  - TASK-2425
  - TASK-2427
modified_files:
  - extensions-rust/foundation/src/lib.rs
  - extensions-rust/foundation/src/tests.rs
  - extensions-rust/foundation/templates/lints.toml
  - docs/foundation.md
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave50
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Both findings live in the extensions-rust/foundation crate: the check/scaffold that silently skips unresolvable workspace members, and the lint template that lacks the unsafe_code policy the workspace itself enforces.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: none
<!-- SECTION:NOTES:END -->
