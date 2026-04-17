---
id: TASK-0084
title: >-
  ARCH-8: duckdb/src/lib.rs mixes trait impls, factories, provider, and 270
  lines of tests
status: To Do
assignee: []
created_date: '2026-04-17 11:32'
updated_date: '2026-04-17 12:07'
labels:
  - rust-codereview
  - arch
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/lib.rs:1`

**What**: lib.rs is 391 lines with DuckDbExtension, DuckDbProvider, trait impls, and extensive inline test module covering ingestor, error types, mock ingestors.

**Why it matters**: Violates thin lib.rs principle; tests belong in their respective modules.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Move MockIngestor/ingestor_error_tests to extensions/duckdb/src/ingestor.rs
- [ ] #2 Move DbError tests to error.rs
- [ ] #3 Move DuckDbProvider tests next to provider definition or into an integration test file
<!-- AC:END -->
