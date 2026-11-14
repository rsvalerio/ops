---
id: TASK-2195
title: 'TEST-23: the provider schema''s 15-field list can silently drift from CoverageRow - nothing binds them'
status: Done
assignee: []
created_date: '2026-09-08 07:14'
updated_date: '2026-09-09 18:14'
labels:
  - code-review-rust
  - tests
dependencies: []
parent_task_id: 'TASK-2240'
modified_files:
  - extensions-rust/test-coverage/src/provider.rs
  - extensions-rust/test-coverage/src/tests/provider.rs
priority: medium
ordinal: 108000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/provider.rs:20` (`schema`), `extensions-rust/test-coverage/src/parse.rs:21` (`CoverageRow`)

**What**: the `DUP-3 / TASK-1555` comment on `CoverageProvider::schema` states that "the field list mirrors `CoverageRow`'s struct layout. Adding a new metric flows through one struct edit + this list". Only half of that is enforced. Adding or renaming a field on `CoverageRow` breaks compilation in `query_coverage_files` (the projection constructs the struct literally), but the `DataProviderSchema` `data_field!` list in `schema()` is a free-standing `Vec` of string literals — it compiles unchanged and keeps advertising the old 15 fields while the rows carry 16.

The existing guard, `coverage_provider_schema_has_fields` in `src/tests/provider.rs:12`, is not that binding either: it hardcodes `assert_eq!(schema.fields.len(), 15)` and then `contains()`-checks the same 15 literals it is guarding. It restates the schema rather than comparing it to the row type, so it passes unchanged after a `CoverageRow` edit.

A binding test is cheap: serialize a `CoverageRow` (it derives `Serialize`) and compare the resulting JSON object's key set against `schema().fields.iter().map(|f| f.name)`.

**Why it matters**: `DataProviderSchema` is what downstream consumers use to discover the coverage columns. A silent divergence advertises a column set that no longer matches the rows the provider returns, and the drift surfaces as a missing-key failure in a consumer rather than a compile error here — exactly the failure mode the DUP-3 refactor claimed to have closed.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 A test derives the expected field-name set from a serialized CoverageRow and asserts it equals the name set in CoverageProvider::schema()
- [x] #2 The hardcoded 15-literal restatement in coverage_provider_schema_has_fields is replaced or reduced so that adding a CoverageRow field fails the suite rather than passing silently
- [x] #3 Field ordering expectations, if any, are stated explicitly in the test rather than assumed

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-2240. coverage_provider_schema_has_fields replaced by coverage_provider_schema_fields_match_covered_row_serialization: expected set derived from a serialized CoverageRow, set equality with schema().fields names; ordering explicitly not asserted (name-bound consumers), documented in the test.
<!-- SECTION:NOTES:END -->
