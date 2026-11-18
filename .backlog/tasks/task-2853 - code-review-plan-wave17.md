---
id: TASK-2853
title: 'code-review-plan-wave17'
status: To Do
assignee: []
created_date: '2026-11-18 20:32'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-wave
dependencies:
  - TASK-2829
  - TASK-2830
  - TASK-2831
  - TASK-2833
  - TASK-2834
  - TASK-2835
  - TASK-2836
  - TASK-2760
  - TASK-2761
  - TASK-2762
  - TASK-2763
  - TASK-2764
modified_files:
  - extensions-rust/metadata/src/ingestor.rs
  - extensions-rust/metadata/src/lib.rs
  - extensions-rust/metadata/src/tests/payload_cap.rs
  - extensions/sqlite/Cargo.toml
  - extensions/sqlite/src/connection.rs
  - extensions/sqlite/src/error.rs
  - extensions/sqlite/src/ingestor.rs
  - extensions/sqlite/src/lib.rs
  - extensions/sqlite/src/sql/ingest/mod.rs
  - extensions/sqlite/src/sql/ingest/orchestrator.rs
  - extensions/sqlite/src/sql/ingest/sidecar.rs
  - extensions/sqlite/src/sql/ingest/sql.rs
  - extensions/sqlite/src/sql/query/coverage.rs
  - extensions/sqlite/src/sql/query/deps.rs
  - extensions/sqlite/src/sql/query/helpers.rs
  - extensions/sqlite/src/sql/query/loc.rs
  - extensions/sqlite/src/sql/validation.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave17
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Grouping: sqlite ingest atomicity and queries, plus metadata ingestor (shared half-load root cause)
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2839 (extensions/sqlite/src/connection.rs, extensions/sqlite/src/sql/ingest/orchestrator.rs)
<!-- SECTION:NOTES:END -->
