---
id: TASK-1689
title: code-review-plan-wave147
status: To Do
assignee:
  - code-review-wave
created_date: '2026-08-26 21:17'
updated_date: '2026-08-26 21:18'
labels:
  - code-review-wave
dependencies:
  - TASK-1681
modified_files:
  - Cargo.toml
  - crates/core/src/expand.rs
  - crates/runner/src/command/build.rs
  - crates/runner/src/command/tests/data.rs
  - extensions-rust/about/src/coverage_provider.rs
  - extensions-rust/about/src/query.rs
  - extensions-rust/loc/src/tests.rs
  - extensions-rust/metadata/src/ingestor.rs
  - extensions-rust/test-coverage/src/ingestor.rs
  - extensions-rust/test-coverage/src/tests.rs
  - extensions/about/src/manifest_cache.rs
  - extensions/duckdb/src/connection.rs
  - extensions/duckdb/src/schema.rs
  - extensions/duckdb/src/sql/ingest/orchestrator.rs
  - extensions/duckdb/src/sql/ingest/sql.rs
  - extensions/duckdb/src/sql/query/helpers.rs
  - extensions/duckdb/src/sql/query/loc.rs
  - extensions/tokei/src/tests.rs
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave147
<!-- SECTION:DESCRIPTION:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
`significant_drop_tightening` alone: lock-guard lifetime in the duckdb/metadata ingest paths - a concurrency concern disjoint from the rest of the sweep.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-1683 [wave141] 2 files (Cargo.toml, extensions-rust/about/src/query.rs); TASK-1684 [wave142] 5 files (Cargo.toml, crates/core/src/expand.rs...); TASK-1685 [wave143] 4 files (Cargo.toml, extensions-rust/about/src/query.rs...); TASK-1686 [wave144] 10 files (Cargo.toml, crates/runner/src/command/build.rs...); TASK-1687 [wave145] 5 files (Cargo.toml, crates/core/src/expand.rs...); TASK-1688 [wave146] 3 files (Cargo.toml, extensions/about/src/manifest_cache.rs...)

Every wave in this batch edits the `# --- Temporary allows ---` block in the root `Cargo.toml`, so a one-line merge there is expected on each landing.
<!-- SECTION:NOTES:END -->
