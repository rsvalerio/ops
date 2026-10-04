---
id: TASK-2416
title: 'code-review-plan-wave41'
status: Done
assignee: []
created_date: '2026-10-04 14:52'
updated_date: '2026-10-04 15:51'
labels:
  - code-review-wave
dependencies:
  - TASK-2406
  - TASK-2407
  - TASK-2408
  - TASK-2409
  - TASK-2410
  - TASK-2411
  - TASK-2412
  - TASK-2413
modified_files:
  - extensions/sqlite/src/connection.rs
  - extensions/sqlite/src/error.rs
  - extensions/sqlite/src/ingestor.rs
  - extensions/sqlite/src/schema.rs
  - extensions/sqlite/src/sql/ingest/dir.rs
  - extensions/sqlite/src/sql/ingest/sidecar.rs
  - extensions/sqlite/src/sql/ingest/sql.rs
  - extensions/sqlite/src/sql/query/coverage.rs
  - extensions/sqlite/src/sql/query/helpers.rs
  - extensions/sqlite/src/sql/validation.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave41
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
ops-sqlite: path validation and ingest-dir permission bugs (the two MEDIUMs), error context, dead code and docs. All in one crate, heavily shared files.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: none

Branch: code-review/TASK-2416
Worktree: /home/rsvalerio/projects/.wave-TASK-2416

<!-- SECTION:NOTES:END -->
