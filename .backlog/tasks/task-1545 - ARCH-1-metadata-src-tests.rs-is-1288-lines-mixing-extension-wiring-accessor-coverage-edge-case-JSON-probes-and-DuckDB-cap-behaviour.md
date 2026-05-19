---
id: TASK-1545
title: >-
  ARCH-1: metadata/src/tests.rs is 1288 lines mixing extension wiring, accessor
  coverage, edge-case JSON probes, and DuckDB cap behaviour
status: To Do
assignee:
  - TASK-1576
created_date: '2026-05-19 15:25'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - ARCH
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/metadata/src/tests.rs:1-1288`

**What**: `tests.rs` is 1288 lines and groups four unrelated test concerns into one file:
- Extension/Provider wiring (`metadata_provider_name`, `metadata_provider_returns_valid_json`, `metadata_provider_fails_in_non_cargo_dir`, `metadata_schema_has_expected_fields`, `test_datasource_extension!` macro at top)
- `Metadata`/`Package`/`Dependency`/`Target` accessor coverage (lines ~88-470)
- `metadata_edge_case_tests` submodule (lines 472-1288) holding cap/payload/duplicate-id/duplicate-name/check_metadata_output tests
- DuckDB `query_metadata_raw` / `query_metadata_raw_with_cap` cap-behaviour tests (lines 949-1066)

The file exceeds the ARCH-1 300-line guideline by ~4x. The grouping is `mod metadata_edge_case_tests { ... }` which holds 800+ lines on its own.

**Why it matters**: With four concerns in one file, a reviewer scanning blame on a payload-cap regression must wade past hundreds of accessor tests; conversely, a refactor of the JSON accessor surface drags the cap tests into the diff context. Splitting into `tests/extension_wiring.rs`, `tests/accessors.rs`, `tests/payload_cap.rs`, `tests/edge_cases.rs` (as integration tests under `tests/`, or as separate `mod`s in `lib.rs` `#[cfg(test)]`) restores reviewer scope and reduces the rebuild blast radius — touching one test file no longer recompiles the entire test binary.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 metadata/src/tests.rs is split into multiple focused test files (or sibling test modules) of <400 lines each
- [ ] #2 Each file's purpose is named and visible from the file name / module name (e.g. payload_cap, accessors, edge_cases)
- [ ] #3 Total test coverage and assertion count is preserved; cargo test passes
<!-- AC:END -->
