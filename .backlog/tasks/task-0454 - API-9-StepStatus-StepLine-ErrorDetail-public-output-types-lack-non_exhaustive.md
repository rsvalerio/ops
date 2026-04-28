---
id: TASK-0454
title: >-
  API-9: StepStatus, StepLine, ErrorDetail public output types lack
  #[non_exhaustive]
status: Done
assignee:
  - TASK-0537
created_date: '2026-04-28 05:44'
updated_date: '2026-04-28 16:51'
labels:
  - code-review-rust
  - API
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/core/src/output.rs:40-66`

**What**: `StepStatus`, `StepLine`, `ErrorDetail` are pub and used across crate boundaries (themes, runner, extensions). None are `#[non_exhaustive]`, so adding e.g. `StepStatus::Cancelled` or a field to `ErrorDetail` is a breaking change for every downstream consumer that pattern-matches.

**Why it matters**: The codebase deliberately uses #[non_exhaustive] elsewhere (TASK-0435 for ops_cargo_toml). StepStatus is highly likely to grow variants.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Add #[non_exhaustive] to StepStatus, StepLine, ErrorDetail, plus constructors where field-literal init was previously possible
- [x] #2 All downstream call sites updated; tests cover construction via the new helpers
<!-- AC:END -->
