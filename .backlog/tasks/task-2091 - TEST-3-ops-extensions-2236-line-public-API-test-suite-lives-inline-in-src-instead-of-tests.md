---
id: TASK-2091
title: >-
  TEST-3: ops-extension's 2236-line public-API test suite lives inline in src/
  instead of tests/
status: To Do
assignee:
  - TASK-2241
created_date: '2026-09-07 22:58'
updated_date: '2026-09-08 10:56'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - crates/extension/src/tests.rs
  - crates/extension/src/lib.rs
priority: medium
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `crates/extension/src/tests.rs` (all), declared at `crates/extension/src/lib.rs:42-43`

**What**: The crate's entire suite — 88 `#[test]` fns across 2236 lines — sits in a single `#[cfg(test)] mod tests` in `src/`, outweighing the ~1980 lines of production code it covers. Nearly all of it exercises the *public* API: `DataRegistry` register/provide/schemas, `Context` caching/refresh/deadline, `DataProviderError` rendering, `CommandRegistry` insert/audit, `impl_extension!`/`data_field!` macros, `sort_compiled_extensions`. Only the `SharedError::shares_allocation_with` tests (a `#[cfg(test)] pub(crate)` helper) genuinely need private access.

**Why it matters**: TEST-3 — the dividing line is the surface under test: anything that only touches the public API is an integration test and belongs in `tests/`, which also proves the API is usable from outside the crate and keeps `src/` readable. A 2236-line inline module buries the implementation in the IDE and in every diff.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Public-API tests moved to crates/extension/tests/ integration test file(s); cargo test -p ops-extension runs them from there
- [ ] #2 Only tests needing private access (SharedError::shares_allocation_with) remain in a #[cfg(test)] module beside error.rs
- [ ] #3 No test scenario lost: test count after the move is >= 88 or every dropped test is accounted for as duplicated coverage
<!-- AC:END -->
