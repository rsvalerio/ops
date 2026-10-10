---
id: TASK-2441
title: 'code-review-plan-wave53'
status: To Do
assignee: []
created_date: '2026-10-10 14:31'
updated_date: '2026-10-10 14:32'
labels:
  - code-review-wave
dependencies:
  - TASK-2428
  - TASK-2435
modified_files:
  - extensions-rust/metadata/src/ingestor.rs
  - extensions/sqlite/src/error.rs
  - extensions/create-review-tasks/src/backlog.rs
  - extensions/create-review-tasks/src/lib.rs
  - crates/backlog/src/store.rs
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
code-review-plan-wave53
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->

<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
Both are error-fidelity fixes: io_at/read_staged_payload flattening or dropping the IO error source and path in the metadata ingestor, and create-review-tasks allocating ids through tolerant walkers that skip unreadable directories. Same concern: a failure the caller needs to see is squashed into a partial or misleading result.
<!-- SECTION:PLAN:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Overlaps: TASK-2439 (extensions/sqlite/src/error.rs)
<!-- SECTION:NOTES:END -->
