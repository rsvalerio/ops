---
id: TASK-2198
title: 'API-14: ops-test-coverage public consts lack doc summaries and 19 crate-internal items are pub despite lib.rs claiming they were demoted'
status: Done
assignee: []
created_date: '2026-09-08 07:15'
updated_date: '2026-09-10 16:26'
labels:
  - code-review-rust
  - api
dependencies: []
parent_task_id: 'TASK-2247'
modified_files:
  - extensions-rust/test-coverage/src/lib.rs
  - extensions-rust/test-coverage/src/parse.rs
  - extensions-rust/test-coverage/src/provider.rs
  - extensions-rust/test-coverage/src/subprocess.rs
  - extensions-rust/test-coverage/src/views.rs
  - extensions-rust/test-coverage/src/ingestor.rs
priority: low
ordinal: 111000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs:44` (the four consts), plus `parse.rs`, `provider.rs`, `subprocess.rs`, `views.rs`, `ingestor.rs`

**What**: two related visibility/doc gaps.

1. **No doc summaries on the crate's real public surface.** `pub const NAME`, `DESCRIPTION`, `SHORTNAME` and `DATA_PROVIDER_NAME` in `lib.rs:44-48` carry no `///` at all. They are genuinely exported (the modules holding everything else are private), so they are the crate's public API alongside `CoverageExtension` and `load_coverage`, both of which *are* documented.

2. **`pub` on items that are crate-internal.** `lib.rs:38` asserts: "API-9 / TASK-1602: `flatten_coverage_json` / `collect_coverage` have no external callers either; demoted to `pub(crate)` in `parse.rs`." They were not. `parse.rs:237` and `parse.rs:361` are both plain `pub fn`. In total 19 items across the five private modules are `pub` rather than `pub(crate)`:

- `subprocess.rs:22,30,46,71,95,115,132` — `CARGO_LLVM_COV_TIMEOUT`, `LLVM_COV_ARGS`, `llvm_cov_argv`, `llvm_cov_timeout`, `run_cargo_llvm_cov`, `format_cargo_exit`, `check_llvm_cov_output`
- `parse.rs:21,100,237,310,328,361,380` — `CoverageRow` (whose *fields* are correctly `pub(crate)`), `DriftTracker`, `flatten_coverage_json`, `format_stderr_diagnostic`, `has_parseable_coverage_data`, `collect_coverage`, `collect_coverage_with`
- `provider.rs:10,61,94` — `CoverageProvider`, `query_coverage_files`, `provide_from_db`
- `views.rs:12,28` — `coverage_files_create_sql`, `coverage_summary_view_sql`
- `ingestor.rs:12` — `CoverageIngestor`

**Why it matters**: the effective visibility is correct only by accident of `lib.rs`'s private `mod` declarations. Making any of those modules `pub` — or re-exporting one — instantly exports 19 items that the crate's own comments say are internal, including `CoverageRow` with private fields and the raw `check_llvm_cov_output`. It also makes the in-code documentation actively wrong, so a reader trusting the `lib.rs:38` comment reasons about a visibility boundary that does not exist. `unreachable_pub` flags exactly this class.

Same shape as TASK-2163 (`ops-about-rust`) and TASK-2130 (`ops-duckdb`), but a disjoint file set.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 NAME, DESCRIPTION, SHORTNAME and DATA_PROVIDER_NAME each carry a one-line doc summary
- [x] #2 Every item in the private parse/provider/subprocess/views/ingestor modules that has no out-of-crate caller is pub(crate) rather than pub
- [x] #3 The lib.rs API-9 / TASK-1602 comment matches the code after the change, or is corrected
- [x] #4 cargo clippy passes with unreachable_pub enabled for this crate (or the lint is added to the workspace policy separately)

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
AC2/AC4 substituted: clippy::redundant_pub_crate (nursery, denied workspace-wide) forbids the literal pub(crate) spelling inside the five private modules — the repo convention (documented in ops-cargo-toml/src/lib.rs and now in this crate lib.rs) spells such items pub, with the private mod declarations as the real boundary. unreachable_pub therefore cannot be enabled (it flags the sanctioned pub spelling); AC3 satisfied by correcting the API-9/TASK-1602 comment to describe reality, and the boundary is now stated in the lib.rs module docs.
<!-- SECTION:NOTES:END -->
