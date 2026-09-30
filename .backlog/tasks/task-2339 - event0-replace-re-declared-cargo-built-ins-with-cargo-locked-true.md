---
id: TASK-2339
title: 'event0: replace re-declared cargo built-ins with [cargo] locked = true'
status: In Progress
assignee: []
created_date: '2026-09-28 15:29'
updated_date: '2026-09-29 17:14'
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
- [x] #1 event0 .ops.toml no longer re-declares the 9 cargo built-ins
- [x] #2 event0 sets [cargo] locked = true and its CI still passes

<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
event0 branch chore/cargo-locked-switch (uncommitted): .ops.toml drops the build/check/clippy/test/test-ignored/next/next-ignored/test-doc re-declarations and sets [cargo] locked = true (event0 is on ops 0.75.0, which has TASK-2323 incl. 78680b88). doc stays: it is a genuine override (doc-public + doc-private legs with RUSTDOCFLAGS=-D warnings), not a copy of the built-in; its legs and release-lint just lose their now-redundant --locked. ops --dry-run shows --locked on all 11 cargo invocations. event0 has no GitHub CI; its gate ops verify-check passes locally (9/9). Close once the event0 change lands.

Landing via rsvalerio/event0#19 (branch chore/ops-locked-config).

<!-- SECTION:NOTES:END -->
