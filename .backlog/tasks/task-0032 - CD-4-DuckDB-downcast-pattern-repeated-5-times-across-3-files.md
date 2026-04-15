---
id: TASK-0032
title: 'CD-4: DuckDB downcast pattern repeated 5 times across 3 files'
status: Done
assignee: []
created_date: '2026-04-14 19:36'
updated_date: '2026-04-15 09:56'
labels:
  - rust-code-duplication
  - DUP-3
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**Location**: extensions/about/src/lib.rs:203-209, extensions-rust/about/src/identity.rs:329-334 + 339-342 + 360-363, extensions-rust/about/src/query.rs:73-75
**Anchor**: fn query_loc_from_db, fn query_dependency_count, fn query_coverage_and_languages, fn enrich_from_db, fn duckdb_handle
**Impact**: The DuckDB downcast pattern ctx.db.as_ref().and_then(|h| h.as_any().downcast_ref::<ops_duckdb::DuckDb>()) is repeated 5 times across 3 files. Each occurrence acquires the DuckDb handle from Context through the same 2-line Any-downcast chain. In extensions-rust/about/src/identity.rs alone, it appears 3 times in adjacent helper functions (query_loc_from_db, query_dependency_count, query_coverage_and_languages).

Fix: add a Context::duckdb() -> Option<&DuckDb> convenience method (behind a feature gate), or extract a local get_db(ctx) helper in each crate. The identity.rs file would benefit most since it has 3 adjacent occurrences.

DUP-3: 5 occurrences of repeated pattern.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 DuckDB handle acquisition is a single call per use site, not a repeated 2-line downcast chain
<!-- AC:END -->
