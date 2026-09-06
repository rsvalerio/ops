---
id: TASK-1944
title: >-
  ARCH-1: test-coverage/src/tests.rs is 940 lines mixing six concerns in a flat
  module, against the src/tests/ convention two sibling extensions already
  follow
status: Done
assignee:
  - TASK-2000
created_date: '2026-08-27 15:48'
updated_date: '2026-08-28 15:53'
labels:
  - code-review-rust
  - architecture
dependencies: []
modified_files:
  - extensions-rust/test-coverage/src/tests.rs
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/tests.rs` (940 lines)

**What**: the crate's production code was deliberately split by concern under ARCH-1 / TASK-1559 — `lib.rs` went from 412 lines to 100, and `subprocess.rs`, `parse.rs`, `provider.rs`, `ingestor.rs`, `views.rs` each own one thing. The test module was not part of that split and has kept growing. It is now the largest tests.rs in the workspace at 940 lines and holds six unrelated groups behind hand-written banner comments:

- extension trait wiring and provider schema (lines 11-55)
- `flatten_coverage_json` fixtures and behaviour, including the dedup, drift, and malformed-record cases (57-215, and again 566-700)
- `check_llvm_cov_output` and `format_stderr_diagnostic` subprocess tests (317-521)
- argv regression guards for `LLVM_COV_ARGS` / `llvm_cov_argv` (522-560)
- DuckDB ingest and `coverage_summary` view integration tests (191-316, 800-940)
- `load_coverage` and `query_coverage_files` round-trips (760-800)

The grouping is already broken in practice: the `flatten_coverage_json` tests appear in two separate blocks 350 lines apart, split by the DuckDB and subprocess sections, because new cases were appended at the end rather than filed with their siblings.

The project convention for this exact situation is a `src/tests/` directory with one file per concern. Two sibling extensions already do it: `extensions-rust/cargo-toml/src/tests/` (extension.rs, find_root.rs, inheritance.rs, parse_edge.rs, provider.rs, types.rs) and `extensions-rust/metadata/src/tests/` (accessors.rs, duplicates.rs, edge_cases.rs, payload_cap.rs, wiring.rs). Both got there through the same finding — TASK-1567 and TASK-1670 split `tools/src/tests.rs` (1196 lines) and `deps/src/tests.rs` (1589 lines) on identical grounds.

Note that `ingestor.rs` and `views.rs` also carry their own inline `#[cfg(test)]` modules, so test placement in this crate is currently split three ways with no stated rule. The split should settle that too.

**Why it matters**: at this size the banner comments are the only navigation, and they have already failed to keep related tests together. Mirroring the production module layout (`tests/parse.rs`, `tests/subprocess.rs`, `tests/provider.rs`, `tests/ingest.rs`, `tests/wiring.rs`) makes the home for a new test obvious and matches what the rest of the workspace does.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 src/tests.rs is replaced by a src/tests/ directory with one file per concern, mirroring the production module names (parse, subprocess, provider, ingest, views, wiring)
- [x] #2 No single file in src/tests/ exceeds roughly 300 lines, and every test lives with the concern it exercises rather than in append order
- [x] #3 The three-way split of test placement is settled: either the inline modules in ingestor.rs and views.rs move under src/tests/, or a one-line note in the tests module states why they stay inline
- [x] #4 The full crate test suite passes with no test removed or renamed as part of the move
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
src/tests.rs (940 lines) is replaced by src/tests/ with mod.rs (shared fixtures + placement rule), wiring.rs, parse.rs, parse_edge.rs, collect.rs, subprocess.rs, provider.rs, ingest.rs, views.rs. Largest file is views.rs at 257 lines; parse tests are split into parse.rs (well-formed/dedup) and parse_edge.rs (malformed input) purely for size, mirroring the sibling cargo-toml/src/tests/parse_edge.rs precedent. Three-way placement settled: views.rs inline tests moved under src/tests/views.rs; ingestor.rs keeps its inline module because its tests bind the module-private PIPELINE const, and tests/mod.rs states that rule. All 62 crate tests pass, none removed or renamed.
<!-- SECTION:NOTES:END -->
