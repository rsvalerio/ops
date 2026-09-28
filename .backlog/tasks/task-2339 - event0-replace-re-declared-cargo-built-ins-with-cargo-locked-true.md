---
id: TASK-2339
title: 'event0: replace re-declared cargo built-ins with [cargo] locked = true'
status: Triage
assignee: []
created_date: '2026-09-28 15:29'
labels:
  - ops-alignment
  - ci
dependencies: []
modified_files: []
priority: low
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
TASK-2323 added a single [cargo] locked switch to ops. event0 re-declares 9 built-ins (build/check/clippy/doc/test/test-ignored/next/next-ignored/test-doc) only to add --locked. Once event0 is on an ops release containing TASK-2323, drop those re-declarations and add [cargo]\nlocked = true. Work lives in the event0 repo, outside ops; track it here so it is not lost.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 event0 .ops.toml no longer re-declares the 9 cargo built-ins
- [ ] #2 event0 sets [cargo] locked = true and its CI still passes
<!-- AC:END -->
