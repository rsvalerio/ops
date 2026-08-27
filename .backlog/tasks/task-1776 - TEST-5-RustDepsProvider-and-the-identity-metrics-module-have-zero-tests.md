---
id: TASK-1776
title: 'TEST-5: RustDepsProvider and the identity metrics module have zero tests'
status: Triage
assignee: []
created_date: '2026-08-27 11:22'
labels:
  - code-review-rust
  - test-quality
dependencies: []
modified_files:
  - extensions-rust/about/src/deps_provider.rs
  - extensions-rust/about/src/identity/metrics.rs
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/about/src/deps_provider.rs` (whole file, 40 lines), `extensions-rust/about/src/identity/metrics.rs` (whole file, 91 lines)

**What**: Two of the crate's five modules have no `#[cfg(test)]` block, and the crate has no `tests/` directory, so nothing exercises them at any level.

`deps_provider.rs` — `RustDepsProvider::name` and `RustDepsProvider::provide` are entirely untested. `provide` has three distinct behaviours, none covered:
- no `DuckDb` in the context → serialise `ProjectDependencies::default()` (`:19-21`);
- `query_crate_deps` fails → `query_or_warn` logs and falls back to an empty map (`:26-31`);
- success → per-crate `(String, Vec<(String, String)>)` rows are mapped into `UnitDeps` and wrapped in `ProjectDependencies` (`:32-38`).

The fallback arm is the crate's documented ERR-2 / TASK-0376 contract ("a DuckDB schema/migration error here used to surface as an empty deps list with no signal"), and there is no test pinning that the warn fires or that the fallback shape is a valid empty `ProjectDependencies` rather than an error.

`identity/metrics.rs` — `query_identity_metrics` and its four helpers (`query_dependency_count`, `query_coverage_and_languages`, `query_loc_from_db`) are untested. The all-`None` fallback when `get_db` returns `None` (`:22-30`) and the `lines_count > 0` guard at `:63-67` (which decides whether a real 0%-coverage project is reported as `Some(0.0)` or as `None`) are both uncovered branches with observable user-facing meaning.

Contrast: the sibling providers are well covered — `units.rs` has 8 tests, `query.rs` 17, `identity/mod.rs` 12, `coverage_provider.rs` 4.

The tooling for these tests already exists in the crate's dev-dependencies and is used elsewhere: `DuckDb::open_in_memory()` plus a seeded broken schema (`coverage_provider.rs:205-222`), `ops_about::test_support::TracingBuf` for warn capture, and `Context::test_context`.

**Why it matters**: TEST-5 — every public API function needs at least one test. `RustDepsProvider` is registered into the extension registry at `lib.rs:62-65` and feeds the `project_dependencies` about subpage, so a regression there ships silently. The untested branches are exactly the degraded-mode paths (`DuckDb` absent, query failed) that are hardest to notice in manual use because they succeed with empty data.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 RustDepsProvider::provide has tests for all three paths: no DuckDb in context, query_crate_deps failure (asserting the query_or_warn warn fires via TracingBuf), and a successful multi-crate result mapped into UnitDeps
- [ ] #2 RustDepsProvider::name is asserted to equal PROVIDER_NAME
- [ ] #3 query_identity_metrics has a test for the all-None fallback when no DuckDb is present
- [ ] #4 The lines_count > 0 guard in query_coverage_and_languages is covered by tests for both a zero-line project (coverage_percent is None) and a non-zero project
<!-- AC:END -->
