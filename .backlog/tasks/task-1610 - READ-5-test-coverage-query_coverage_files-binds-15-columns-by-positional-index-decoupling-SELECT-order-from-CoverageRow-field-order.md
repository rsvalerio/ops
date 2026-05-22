---
id: TASK-1610
title: >-
  READ-5: test-coverage query_coverage_files binds 15 columns by positional
  index, decoupling SELECT order from CoverageRow field order
status: Done
assignee:
  - TASK-1635
created_date: '2026-05-22 06:48'
updated_date: '2026-05-22 10:15'
labels:
  - code-review-rust
  - readability
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/provider.rs:61-92`

**What**: `query_coverage_files` lists 15 columns in a stringified SELECT, then unpacks each via numeric position (`row.get::<_, String>(0)?` … `row.get::<_, f64>(14)?`). The mapping from SELECT order to `CoverageRow` field is invisible to the compiler. All 8 i64 columns and all 4 f64 columns are interchangeable from the type checker's perspective.

**Why it matters**: Reordering the SELECT, inserting a column, or reordering struct fields would silently corrupt the JSON output (e.g., swapping `lines_count`/`lines_covered` still compiles and typechecks). The `CoverageRow` consolidation tracked in TASK-1555 made the schema single-source, but the projection step here is still index-coupled. Silent data corruption risk on schema drift. (READ-5 with FN-4 nuance.)
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Rows are projected either via column-name lookup (row.get_by_name) or via a compile-time-checked macro that ties SELECT order to CoverageRow field order
- [ ] #2 A regression test demonstrates that swapping two same-typed columns in the SELECT is either caught at compile time or produces a failing assertion
- [ ] #3 CoverageRow field reorderings either fail to compile or trigger a test failure
<!-- AC:END -->
