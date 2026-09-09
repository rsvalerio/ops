---
id: TASK-2234
title: 'code-review-plan-wave0'
status: Done
assignee: []
created_date: '2026-09-08 10:50'
updated_date: '2026-09-08 20:00'
labels:
  - code-review-wave
dependencies:
  - TASK-2108
  - TASK-2140
  - TASK-2141
  - TASK-2176
  - TASK-2179
  - TASK-2188
  - TASK-2156
  - TASK-2129
  - TASK-2134
  - TASK-2122
modified_files:
  - crates/runner/src/command/builtins.rs
  - extensions-rust/deps/src/parse/deny.rs
  - extensions-rust/deps/src/parse/upgrade.rs
  - extensions-rust/metadata/src/ingestor.rs
  - extensions-rust/metadata/src/lib.rs
  - extensions/config-checkers/src/lib.rs
  - extensions/duckdb/src/sql/ingest/orchestrator.rs
  - extensions/hook-common/src/git.rs
  - extensions/hook-common/src/install.rs
  - extensions/hook-common/src/lib.rs
  - extensions/run-before-commit/src/lib.rs
  - extensions/run-before-push/src/lib.rs
  - extensions/tokei/src/ingestor.rs
  - extensions/tokei/src/tests.rs
ordinal: 140000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave0: Guards that do not fire on the path that actually runs, and degraded input accepted as complete
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2247 wave13 (6 files: extensions-rust/deps/src/parse/upgrade.rs ...); TASK-2248 wave14 (6 files: extensions-rust/deps/src/parse/deny.rs ...); TASK-2241 wave7 (2 files: extensions/run-before-commit/src/lib.rs ...); TASK-2237 wave3 (1 file: extensions/config-checkers/src/lib.rs); TASK-2243 wave9 (1 file: extensions/tokei/src/tests.rs); TASK-2246 wave12 (1 file: extensions-rust/metadata/src/lib.rs); TASK-2249 wave15 (1 file: extensions/hook-common/src/install.rs)

Branch: code-review/TASK-2234
<!-- SECTION:NOTES:END -->
