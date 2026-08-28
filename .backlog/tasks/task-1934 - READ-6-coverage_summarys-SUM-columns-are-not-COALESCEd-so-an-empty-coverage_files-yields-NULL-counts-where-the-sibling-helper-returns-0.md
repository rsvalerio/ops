---
id: TASK-1934
title: >-
  READ-6: coverage_summary's SUM columns are not COALESCEd, so an empty
  coverage_files yields NULL counts where the sibling helper returns 0
status: To Do
assignee:
  - TASK-2000
created_date: '2026-08-27 15:46'
updated_date: '2026-08-28 14:14'
labels:
  - code-review-rust
  - readability
dependencies: []
modified_files:
  - extensions-rust/test-coverage/src/views.rs
  - extensions-rust/test-coverage/src/tests.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/views.rs:16-43`

**What**: `coverage_summary_view_sql` builds an ungrouped aggregate over `coverage_files`. The four percentage columns are guarded — `CASE WHEN SUM(lines_count) > 0 THEN ... ELSE 0.0 END` yields 0.0 when the SUM is NULL, because NULL > 0 is NULL and falls to the ELSE arm. The ten raw count columns are not guarded:

    SUM(lines_count) AS lines_count,
    SUM(lines_covered) AS lines_covered,
    SUM(functions_count) AS functions_count,
    ... (and the regions_* / branches_* counts and notcovered columns)

An ungrouped aggregate over zero rows returns exactly one row, and SUM over zero rows is NULL. So against an empty `coverage_files` table, `SELECT lines_count FROM coverage_summary` returns a single row whose value is NULL, not 0. Every consumer that decodes that column as a non-nullable integer gets a decode failure rather than a zero; a consumer decoding it into JSON gets `null` where the schema promises an int.

The empty-table state is reachable: `views::coverage_summary_view_sql` is applied by `CoverageIngestor::load` unconditionally, and the view outlives any particular ingest — a run that skipped every record (all filenames missing or non-string, the TASK-0984 path in `parse::build_record`) leaves the table created and empty with the view already in place. The `table_has_data` gate in the provider path guards the provider, not the view, and `coverage_summary` is queryable by anything holding the DuckDB handle.

Cross-crate note: the sibling aggregate for the same data, `coverage_col_select` in `extensions/duckdb/src/sql/query/helpers.rs:87-96`, wraps exactly these sums — `COALESCE(SUM(lines_count), 0), COALESCE(SUM(lines_covered), 0)` — and guards the percentage with the same CASE. That crate already made the decision; this view is the outlier and no test exercises the empty-table case (`coverage_summary_view_handles_zero_counts` in tests.rs writes a fixture with one all-zero row, which is a different thing entirely — one row of zeros sums to 0, not NULL).

**Why it matters**: the view's contract is "totals across all files", and the empty case is the one boundary where it silently returns a different type than every other case. Fixing it is a one-token change per column plus a test that queries the view against a table with no rows.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every non-percentage SUM column in coverage_summary_view_sql is wrapped in COALESCE(..., 0), matching coverage_col_select in the duckdb extension
- [ ] #2 A test loads the schema with an empty coverage_files table, queries every column of coverage_summary, and asserts each count decodes as 0 rather than failing or returning null
- [ ] #3 The existing coverage_summary_view_handles_zero_counts test is kept and its distinct intent (one all-zero row, not zero rows) is noted so the two are not later merged
<!-- AC:END -->
