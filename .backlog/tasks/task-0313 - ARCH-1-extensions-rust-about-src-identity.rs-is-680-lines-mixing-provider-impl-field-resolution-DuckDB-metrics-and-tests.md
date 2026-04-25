---
id: TASK-0313
title: >-
  ARCH-1: extensions-rust/about/src/identity.rs is 680 lines mixing provider
  impl, field resolution, DuckDB metrics, and tests
status: Done
assignee:
  - TASK-0326
created_date: '2026-04-24 08:53'
updated_date: '2026-04-25 13:12'
labels:
  - rust-code-review
  - architecture
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: extensions-rust/about/src/identity.rs

**What**: 680 lines co-mingling manifest-parsing orchestration, DuckDB metric queries, DataProvider impl, and ~470 lines of inline tests.

**Why it matters**: Crosses the 500-line ARCH-1 red flag; splits naturally into resolver / metrics / provider submodules.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Split into identity/resolver.rs, identity/metrics.rs, identity/mod.rs
- [ ] #2 Tests co-located with their subject; no behavior change
<!-- AC:END -->
