---
id: TASK-2154
title: 'TEST-5: per_crate_units and RustCoverageProvider::provide have no happy-path test'
status: Done
assignee: []
created_date: '2026-09-08 07:03'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-rust
  - tests
dependencies: []
parent_task_id: 'TASK-2240'
modified_files:
  - extensions-rust/about/src/coverage_provider.rs
priority: medium
ordinal: 67000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/coverage_provider.rs:194-300`

**What**: `coverage_provider.rs` has six `#[test]`s, and all six live in `mod cache_tests` (`:301`) exercising the `ProjectCoverageCache` memoization. The provider itself is driven by exactly one test — `non_utf8_workspace_root_skips_per_crate_coverage_with_warn` in the `#[cfg(all(test, unix))] mod tests` at `:565` — which is additionally gated `#[cfg(not(target_os = "macos"))]` and, by construction, takes the early-return branch that *skips* per-crate coverage.

So the following production code has no test at all:

- `per_crate_units` (`:262-300`) — the function that maps DuckDB `query_crate_coverage` rows onto `UnitCoverage`: the `display_names` pre-pass keyed on members that have a coverage row, the `filter_map` that drops members without one, and the `display_names.remove(member)` that consumes each name once. That `remove` is the subtle one: a duplicate member string in `members` would make the second occurrence silently vanish from the output.
- The `RustCoverageProvider::provide` arms for "no `DuckDB` attached → default `ProjectCoverage`" (`:208`), "`cached_query_project_coverage` returned `None` → default" (`:223`), and "manifest failed to load → project total present, per-crate list empty" (`:200-206`, `:252`).
- The `CoverageStats::new(p.lines_percent, p.lines_covered, p.lines_count)` argument order at `:226` and `:294` — a transposition here is invisible to every current test.

The sibling providers are all covered on these paths: `deps_provider.rs` has `provide_without_duckdb_yields_empty_dependencies`, `provide_warns_and_falls_back_when_the_query_fails` and `provide_maps_multi_crate_rows_into_unit_deps`; `units.rs` drives `RustUnitsProvider::provide` in four tests. The coverage provider is the one that was left out.

**Why it matters**: `project_coverage` is the data behind the `ops about` coverage subpage. A regression in the row→unit mapping (wrong stat order, a member silently dropped, an empty per-crate table where rows exist) renders as plausible-looking output, not as a failure, and nothing in CI would go red. On macOS the provider's `provide` is not compiled into any test at all.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test seeds an in-memory DuckDb with `coverage_files` rows for two workspace members and asserts `per_crate_units` returns one `UnitCoverage` per member with the right unit name, member path, and (percent, covered, count) triple in the right order
- [ ] #2 A test pins that a member with no coverage row is omitted from the per-crate list rather than emitted with zeroed stats
- [ ] #3 A test drives `RustCoverageProvider::provide` end to end against a real workspace fixture plus a seeded DuckDb and asserts both the project total and the per-crate table
- [ ] #4 The no-DuckDB and failed-query arms of `provide` are pinned to return a well-formed default `ProjectCoverage` (not an error), matching the shape `deps_provider`'s tests already establish
- [ ] #5 The new tests are platform-independent (not gated on unix / non-macos) and carry `#[serial_test::serial(typed_manifest_cache, project_coverage_cache)]` per the cache modules' contract
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Landed in wave TASK-2240. AC #2 required a bounded production change: query_crate_coverage zero-fills members whose LEFT JOIN matched no coverage_files row, so per_crate_units previously emitted no-data members as 0% rows. It now omits members with lines_count == 0 (no measurable coverage), per the AC intent. New tests: extensions-rust/about/src/coverage_provider.rs provider_tests (6 tests, platform-independent, serial(typed_manifest_cache, project_coverage_cache)).
<!-- SECTION:NOTES:END -->
