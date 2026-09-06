---
id: TASK-1870
title: >-
  TEST-5: three public entry points of ops-duckdb (try_provide_from_db, get_db,
  query_rows_to_json) have no test in the crate
status: Done
assignee:
  - TASK-2006
created_date: '2026-08-27 15:30'
updated_date: '2026-08-28 22:16'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions/duckdb/src/lib.rs
  - extensions/duckdb/src/sql/ingest/sql.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions/duckdb/src/lib.rs:48-68`, `extensions/duckdb/src/sql/ingest/sql.rs:157-180`

**What**: These three are among the most-used items the crate exports, and none of them is exercised by any test in `extensions/duckdb`:

- `try_provide_from_db` (`lib.rs:48`) — 4 downstream call sites (`extensions-rust/metadata`, `extensions-rust/loc`, `extensions-rust/test-coverage`, `extensions/tokei`). Nothing pins the branch contract: DB present → `db_fn` runs and `fallback_fn` does not; DB absent → the reverse; either closure's error maps into `DataProviderError`. A regression that silently always takes the fallback would look like "slower but correct" everywhere.
- `get_db` (`lib.rs:66`) — 8+ downstream call sites across `extensions/about` and `extensions-rust/about`. The downcast-from-`dyn DuckDbHandle` is exactly the kind of thing that returns `None` after an unrelated refactor; every caller degrades silently via `let Some(db) = … else { return … }`.
- `query_rows_to_json` (`sql/ingest/sql.rs:157`) — 3 downstream call sites (`extensions/tokei`, `extensions-rust/loc`, `extensions-rust/test-coverage`). The sibling tests in that file cover `table_has_data`, `table_exists`, and `create_table_from_json_sql`, but the row→JSON mapper, the empty-result shape (`Array([])` vs `Null`), and the row-mapper error path are untested.

The crate's own `lib.rs` test module covers `DuckDb::open*`, `init_schema`, `upsert_data_source`, `lock`, and `DuckDbProvider::provide` — the helpers above were simply skipped.

**Why it matters**: TEST-5. All three sit at the boundary between this crate and every consumer of it, and all three fail *softly* (fallback, `None`, empty array) rather than loudly, so a break shows up as missing data on the about page rather than as a red test.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 try_provide_from_db has tests for both branches (ctx.db set → db_fn wins; ctx.db None → fallback_fn wins) and for error mapping into DataProviderError
- [x] #2 get_db has a test asserting Some for a Context carrying a DuckDb handle and None for one that does not
- [x] #3 query_rows_to_json has tests for a populated table, an empty result set, and a row_mapper that returns Err
- [x] #4 Assertions check concrete values, not just is_ok()
<!-- AC:END -->
