---
id: TASK-2235
title: 'code-review-plan-wave1'
status: Done
assignee: []
created_date: '2026-09-08 10:50'
updated_date: '2026-09-09 18:22'
labels:
  - code-review-wave
dependencies:
  - TASK-2225
  - TASK-2109
  - TASK-2143
  - TASK-2177
  - TASK-2092
  - TASK-2170
  - TASK-2206
  - TASK-2209
modified_files:
  - crates/backlog/src/cmd/edit.rs
  - crates/backlog/src/cmd/wave.rs
  - extensions-rust/cargo-toml/src/lib.rs
  - extensions-rust/cargo-toml/src/workspace_root.rs
  - extensions-rust/loc/src/lib.rs
  - extensions-terraform/plan/src/lib.rs
  - extensions/duckdb/src/sql/ingest/dir.rs
  - extensions/text-fixers/src/atomic.rs
  - extensions/text-fixers/src/discovery.rs
ordinal: 141000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave1: Symlink/TOCTOU path resolution, atomic writes, and unbounded/untimed resource use
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2248 wave14 (5 files: extensions-rust/cargo-toml/src/lib.rs ...); TASK-2247 wave13 (4 files: extensions-rust/cargo-toml/src/lib.rs ...); TASK-2237 wave3 (2 files: extensions/text-fixers/src/atomic.rs ...); TASK-2242 wave8 (2 files: crates/backlog/src/cmd/edit.rs ...); TASK-2246 wave12 (2 files: crates/backlog/src/cmd/edit.rs ...); TASK-2241 wave7 (1 file: extensions-terraform/plan/src/lib.rs); TASK-2244 wave10 (1 file: extensions/duckdb/src/sql/ingest/dir.rs)

Branch: code-review/TASK-2235
<!-- SECTION:NOTES:END -->
