---
id: TASK-1681
title: 'CLIPPY: drop the workspace allow for `significant_drop_tightening` (nursery)'
status: Triage
assignee: []
created_date: '2026-08-25 21:00'
labels:
  - code-review-rust
  - clippy
dependencies: []
priority: medium
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enabling `clippy::nursery` and the panic/arithmetic lints from the photo config surfaced 36 pre-existing sites for this lint family across 17 files. To keep `cargo clippy --workspace --all-features --all-targets -- -D warnings` green, the lint is currently allowed workspace-wide in the `# --- Temporary allows ---` block of the root `Cargo.toml`. That allow is the thing this task removes.

**Lint(s)**: `clippy::significant_drop_tightening`
**Sites**: 36 across 17 files

A lock guard held longer than the code that needs it. Real contention risk in the duckdb ingest path and the metadata ingestor, which hold guards across I/O. Narrow the guard's scope or bind it to a shorter block.

## Scope

| File | Sites |
|---|---|
| `extensions-rust/test-coverage/src/tests.rs` | 5 |
| `extensions-rust/metadata/src/ingestor.rs` | 4 |
| `extensions/duckdb/src/schema.rs` | 4 |
| `extensions/duckdb/src/sql/ingest/sql.rs` | 3 |
| `extensions/tokei/src/tests.rs` | 3 |
| `extensions-rust/about/src/coverage_provider.rs` | 2 |
| `extensions-rust/about/src/query.rs` | 2 |
| `extensions/about/src/manifest_cache.rs` | 2 |
| `extensions/duckdb/src/sql/ingest/orchestrator.rs` | 2 |
| `extensions/duckdb/src/sql/query/loc.rs` | 2 |
| `crates/core/src/expand.rs` | 1 |
| `crates/runner/src/command/build.rs` | 1 |
| `crates/runner/src/command/tests/data.rs` | 1 |
| `extensions-rust/loc/src/tests.rs` | 1 |
| `extensions-rust/test-coverage/src/ingestor.rs` | 1 |
| `extensions/duckdb/src/connection.rs` | 1 |
| `extensions/duckdb/src/sql/query/helpers.rs` | 1 |
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Every site listed in the scope table is either fixed or carries an `#[allow]` at the narrowest scope that works, with a comment giving the reason (docs/clippy.md layer 2 or 3)
- [ ] #2 The line(s) for `significant_drop_tightening` are deleted from the temporary-allow block in the root `Cargo.toml`, and the lint reaches the workspace at `deny`
- [ ] #3 `cargo clippy --workspace --all-features --all-targets -- -D warnings` passes
- [ ] #4 `cargo nextest run --workspace --all-features` and `cargo test --workspace --doc` pass
<!-- AC:END -->
