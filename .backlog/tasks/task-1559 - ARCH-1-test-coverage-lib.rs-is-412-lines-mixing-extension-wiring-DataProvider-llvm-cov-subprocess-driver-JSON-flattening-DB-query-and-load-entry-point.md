---
id: TASK-1559
title: >-
  ARCH-1: test-coverage/lib.rs is 412 lines mixing extension wiring,
  DataProvider, llvm-cov subprocess driver, JSON flattening, DB query, and load
  entry point
status: To Do
assignee:
  - TASK-1577
created_date: '2026-05-19 15:43'
updated_date: '2026-05-19 16:46'
labels:
  - code-review-rust
  - architecture
dependencies: []
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/lib.rs` (412 lines)

**What**: The crate's `lib.rs` bundles six distinct concerns:

1. Extension wiring (`impl_extension!`, name/desc/shortname constants, factory).
2. `CoverageProvider` impl of `DataProvider` (schema with 15 fields + provide).
3. Subprocess driver — `run_cargo_llvm_cov`, `check_llvm_cov_output`, `CARGO_LLVM_COV_TIMEOUT` (lines 89-139).
4. JSON parsing scaffolding — `Section`, `extract_section`, `read_field`, `read_i64_field`, `read_f64_field` (lines 141-208).
5. `flatten_coverage_json` and `collect_coverage` (lines 210-350) — the main flattening + soft-fail policy.
6. DB query + provide_from_db + `load_coverage` (lines 352-412).

**Why it matters**: ARCH-1 — single file at 412 lines mixes wiring (rarely touched) with hot business logic (touched on every llvm-cov schema drift) with DB IO. Reviewers loading `lib.rs` for a one-line wiring change get the full coverage parser in context. Sister extensions (about, deps, etc.) split these into modules; test-coverage's `ingestor.rs` and `views.rs` already exist but lib.rs absorbed too much. Suggested split: `subprocess.rs` (timeout/run/check), `parse.rs` (Section + extract_section + read_field + flatten_coverage_json + collect_coverage), `provider.rs` (CoverageProvider + provide_from_db + query_coverage_files), keep wiring + load_coverage in lib.rs.

<!-- scan confidence: confirmed -->
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 extensions-rust/test-coverage/src/lib.rs is reduced to <200 lines containing only wiring + load_coverage
- [ ] #2 Subprocess driver / JSON parsing / DB-query concerns live in their own submodules
- [ ] #3 All existing tests pass without modification (or only import paths updated)
<!-- AC:END -->
