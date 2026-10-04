---
id: TASK-2383
title: 'TEST-32: name/constant tests assert a value back to its own literal'
status: Done
assignee: []
created_date: '2026-10-04 14:13'
updated_date: '2026-10-04 15:25'
labels:
  - code-review-rust
  - test
dependencies: []
parent_task_id: 'TASK-2420'
modified_files:
  - extensions-rust/loc/src/tests.rs
  - extensions-rust/loc/src/ingestor.rs
  - extensions-rust/loc/src/views.rs
priority: low
ordinal: 1000
dedup_key: 'TEST-32:extensions-rust/loc/src/tests.rs:name-constant-tests'
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/loc/src/tests.rs:31-41`, `tests.rs:731-734`, `extensions-rust/loc/src/ingestor.rs:37-40`, `extensions-rust/loc/src/views.rs:54-74`

**What**: Candidates to inspect:
- `ingestor.rs:38` `rust_loc_ingestor_name`: asserts `RustLocIngestor.name() == "rust-loc"`, where `name()` returns `PIPELINE.name` built from the literal `"rust-loc"` on line 9.
- `tests.rs:732` `rust_loc_provider_name`: asserts `name() == "rust-loc"` where the fn returns `DATA_PROVIDER_NAME = "rust-loc"`; the same fact is already pinned by `test_datasource_extension!(name: "rust-loc", data_provider: "rust-loc")` at tests.rs:27-30.
- `tests.rs:32` `rust_loc_extension_type_is_datasource` and `:39` stack test: assert the values passed verbatim to `impl_extension!`.
- `views.rs:54-74`: `.contains("GROUP BY region")`, `.contains("SUM(code)")` etc. mirror the SQL string literal in the same file; the behavioural check is `rust_loc_summary_view_satisfies_the_shared_summary_query` (tests.rs:797).

**Why it matters**: These pass by construction and add review cost without catching a real regression (TEST-32). Keep the behavioural tests (collect/load cycle, summary view query) and drop or replace the tautologies.

<!-- scan confidence: candidates to inspect -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Each listed test is removed or replaced by a property/behaviour assertion that can fail for a real reason
- [x] #2 Behavioural coverage (collect+load cycle, summary view query) is retained

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Removed the five tautologies (extension type/stack, provider name, ingestor name, view SQL contains_aggregation). Kept views.rs rust_loc_summary_view_sql_quotes_identifiers: it asserts quoting added by the CreateViewSql builder, not text present in the literal. Behavioural coverage retained: rust_loc_collect_and_load_cycle, rust_loc_summary_view_satisfies_the_shared_summary_query.
<!-- SECTION:NOTES:END -->
