---
id: TASK-2839
title: 'code-review-plan-wave3'
status: To Do
assignee: []
created_date: '2026-11-18 20:32'
updated_date: '2026-11-18 20:32'
labels:
  - code-review-wave
dependencies:
  - TASK-2672
  - TASK-2699
  - TASK-2727
  - TASK-2799
  - TASK-2832
modified_files:
  - crates/core/src/config/loader/global.rs
  - crates/core/src/expand.rs
  - crates/core/src/stack/detect.rs
  - crates/core/src/sync.rs
  - crates/runner/src/command/build.rs
  - extensions/about/src/manifest_cache.rs
  - extensions-node/about/src/package_json.rs
  - extensions/sqlite/src/connection.rs
  - extensions/sqlite/src/sql/ingest/orchestrator.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave3
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Grouping: poisoned-lock recovery policy across core/runner/about/node/sqlite (one decision, land together)
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2842 (crates/runner/src/command/build.rs); TASK-2853 (extensions/sqlite/src/connection.rs, extensions/sqlite/src/sql/ingest/orchestrator.rs)
<!-- SECTION:NOTES:END -->
