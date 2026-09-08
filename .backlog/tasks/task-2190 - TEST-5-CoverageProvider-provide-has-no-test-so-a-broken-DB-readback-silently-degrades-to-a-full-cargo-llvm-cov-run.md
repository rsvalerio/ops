---
id: TASK-2190
title: >-
  TEST-5: CoverageProvider::provide has no test, so a broken DB readback
  silently degrades to a full cargo llvm-cov run
status: To Do
assignee:
  - TASK-2240
created_date: '2026-09-08 07:14'
updated_date: '2026-09-08 10:56'
labels:
  - code-review-rust
  - tests
dependencies: []
modified_files:
  - extensions-rust/test-coverage/src/provider.rs
  - extensions-rust/test-coverage/src/tests/provider.rs
priority: medium
ordinal: 103000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
**File**: `extensions-rust/test-coverage/src/provider.rs:15` (`provide`), `extensions-rust/test-coverage/src/provider.rs:94` (`provide_from_db`)

**What**: `src/tests/provider.rs` covers `CoverageProvider::name()`, `schema()` and `query_coverage_files()` only. Neither `DataProvider::provide` nor `provide_from_db` is ever called from a test, so no test exercises the dispatch that production actually runs:

- the `try_provide_from_db` branch selection (DB attached vs not),
- the `provide_via_ingestor("coverage_files", ...)` short-circuit when the table already has rows,
- the wiring from `provide` through to the `query_coverage_files` projection.

The pattern is established elsewhere in the tree — `Context::attach_db` is used exactly this way in `extensions/duckdb/src/lib.rs:209`, `extensions/about/src/code.rs:157` and `extensions-rust/about/src/coverage_provider.rs:628` — so the test costs a tempdir plus an in-memory `DuckDb`, and `src/tests/mod.rs::setup_loaded_db()` already produces both.

**Why it matters**: the two branches of `try_provide_from_db` have wildly different costs. The DB branch is a `SELECT`; the fallback branch is `collect_coverage`, which spawns `cargo llvm-cov --workspace --tests` and blocks for up to fifteen minutes. If the `Arc<dyn DuckDbHandle>` downcast ever stops resolving to `DuckDb`, `provide` does not fail — it silently falls through and runs the entire workspace test suite under instrumentation on every `ops about`. `extensions/duckdb/src/lib.rs:37` documents that exact failure mode as a live hazard (a stray `use ops_extension::DuckDbHandle;` flips every downcast to `None`, "silently, with no error and no compile failure"). This crate has no test that would notice.

Related but distinct: TASK-2154 covers `RustCoverageProvider::provide` in `extensions-rust/about`; this is the separate `CoverageProvider` in `ops-test-coverage`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A test attaches a DuckDb loaded via setup_loaded_db() to a Context and calls CoverageProvider.provide, asserting the two fixture rows come back
- [ ] #2 A test pins that provide takes the DB branch rather than the collect_coverage fallback when a DuckDb handle is attached (no cargo subprocess is spawned)
- [ ] #3 provide_from_db is exercised against a DuckDb whose coverage_files table already holds rows, pinning the provide_via_ingestor short-circuit
<!-- AC:END -->
