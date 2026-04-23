---
id: TASK-0136
title: 'ARCH-8: test_create_sql_validation macro is #[macro_export] from library crate'
status: To Do
assignee: []
created_date: '2026-04-22 21:16'
updated_date: '2026-04-23 06:45'
labels:
  - rust-code-review
  - arch
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/sql/ingest.rs:197-232`

**What**: A test-only macro is annotated #[macro_export], making it part of the public API of ops-duckdb at the crate root.

**Why it matters**: Test scaffolding leaks into the stable API surface; external consumers may start depending on it, locking the shape of test helpers.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Remove #[macro_export] or gate the macro behind a test-only feature / cfg(test)
- [ ] #2 If other workspace crates need it, expose via a dev-dependency or test-helpers crate
<!-- AC:END -->
