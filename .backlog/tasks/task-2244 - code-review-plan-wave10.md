---
id: TASK-2244
title: code-review-plan-wave10
status: To Do
assignee:
  - code-review-wave
created_date: '2026-09-08 10:51'
updated_date: '2026-09-08 10:57'
labels:
  - code-review-wave
dependencies:
  - TASK-2082
  - TASK-2086
  - TASK-2093
  - TASK-2115
  - TASK-2117
  - TASK-2120
  - TASK-2131
  - TASK-2159
  - TASK-2168
modified_files:
  - crates/backlog/src/store.rs
  - crates/core/src/stack/detect.rs
  - crates/runner/src/command/mod.rs
  - crates/runner/src/command/sequential.rs
  - crates/theme/src/configurable.rs
  - crates/theme/src/style/sgr.rs
  - extensions/create-review-tasks/src/backlog.rs
  - extensions/create-review-tasks/src/lib.rs
  - extensions/duckdb/src/sql/ingest/dir.rs
  - extensions/duckdb/src/sql/query/helpers.rs
  - extensions/text-fixers/src/trailing.rs
  - extensions/tokei/src/lib.rs
ordinal: 150000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave10: Redundant work and allocations on per-row/per-file paths
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2247 wave13 (4 files: crates/runner/src/command/mod.rs ...); TASK-2242 wave8 (2 files: extensions/create-review-tasks/src/backlog.rs ...); TASK-2243 wave9 (2 files: crates/backlog/src/store.rs ...); TASK-2246 wave12 (2 files: crates/theme/src/configurable.rs ...); TASK-2248 wave14 (2 files: crates/runner/src/command/mod.rs ...); TASK-2235 wave1 (1 file: extensions/duckdb/src/sql/ingest/dir.rs); TASK-2241 wave7 (1 file: crates/theme/src/configurable.rs); TASK-2249 wave15 (1 file: extensions/create-review-tasks/src/lib.rs)
<!-- SECTION:NOTES:END -->
